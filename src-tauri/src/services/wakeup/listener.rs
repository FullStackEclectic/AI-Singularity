use super::gateway::{DispatchAccountInput, DispatchRequest, RunKind, WakeupGateway};
use super::{WakeupService, WakeupTask};
use crate::db::Database;
use crate::services::event_bus::{DataChangedPayload, EventBus};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// 每个 task 触发节流：内存表，进程重启即重置
static LAST_TRIGGER: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
/// listener 启动一次性标记
static LISTENER_STARTED: OnceLock<()> = OnceLock::new();

fn last_trigger_map() -> &'static Mutex<HashMap<String, Instant>> {
    LAST_TRIGGER.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct WakeupListener;

impl WakeupListener {
    pub fn ensure_started(app: AppHandle) {
        if LISTENER_STARTED.set(()).is_err() {
            return;
        }
        tauri::async_runtime::spawn(async move {
            let mut rx = EventBus::subscribe();
            tracing::info!("[Wakeup Listener] 已启动，监听 EventBus data:changed");
            loop {
                match rx.recv().await {
                    Ok(payload) => {
                        if let Err(err) = handle_event(&app, &payload) {
                            tracing::warn!("[Wakeup Listener] 处理事件失败: {}", err);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "[Wakeup Listener] 错过 {} 条事件（缓冲耗尽），继续监听",
                            skipped
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::warn!("[Wakeup Listener] EventBus 通道关闭，监听器退出");
                        break;
                    }
                }
            }
        });
    }
}

fn handle_event(app: &AppHandle, payload: &DataChangedPayload) -> Result<(), String> {
    // 自循环防护：wakeup gateway 自己发的事件不再回流到 event_driven 任务
    if payload.domain == "wakeup" {
        return Ok(());
    }

    let db = app.state::<Database>();
    let state = WakeupService::load_state(&db)?;
    if !state.enabled {
        return Ok(());
    }

    let now = Utc::now();
    for task in state.tasks.into_iter() {
        if !task.enabled || task.trigger_mode.trim() != "event_driven" {
            continue;
        }
        let Some(config) = task.event_subscribe.as_ref() else {
            continue;
        };
        if !payload.domain.eq_ignore_ascii_case(&config.domain) {
            continue;
        }
        if let Some(action) = config.action.as_deref() {
            if !payload.action.eq_ignore_ascii_case(action) {
                continue;
            }
        }
        if !try_acquire_event_slot(&task.id, config.min_interval_seconds, &task.last_run_at, now) {
            tracing::debug!(
                "[Wakeup Listener] 节流跳过 task_id={} domain={} action={}",
                task.id,
                payload.domain,
                payload.action
            );
            continue;
        }

        dispatch_event_driven(app, task, payload);
    }
    Ok(())
}

fn dispatch_event_driven(app: &AppHandle, task: WakeupTask, payload: &DataChangedPayload) {
    if task.account_id.trim().is_empty()
        || task.model.trim().is_empty()
        || task.command_template.trim().is_empty()
    {
        tracing::debug!(
            "[Wakeup Listener] 跳过：任务缺少账号/模型/命令模板 task_id={}",
            task.id
        );
        return;
    }

    let triggered_by = format!("event:{}/{}", payload.domain, payload.action);
    let req = DispatchRequest {
        kind: RunKind::Event,
        task_template: task.clone(),
        accounts: vec![DispatchAccountInput {
            account_id: task.account_id.clone(),
            task_id: Some(task.id.clone()),
            task_name: if task.name.trim().is_empty() {
                "未命名任务".to_string()
            } else {
                task.name.clone()
            },
        }],
        triggered_by,
        run_id: None,
        mutate_task: true,
    };
    if let Err(err) = WakeupGateway::dispatch(app, req) {
        tracing::warn!(
            "[Wakeup Listener] event_driven 派发失败 task_id={} err={}",
            task.id,
            err
        );
    }
}

fn try_acquire_event_slot(
    task_id: &str,
    min_interval_seconds: u64,
    last_run_at: &Option<String>,
    now: DateTime<Utc>,
) -> bool {
    // 1. 内存表节流（进程内强约束）
    let now_instant = Instant::now();
    let interval = Duration::from_secs(min_interval_seconds.max(60));
    if let Ok(mut guard) = last_trigger_map().lock() {
        if let Some(prev) = guard.get(task_id) {
            if now_instant.duration_since(*prev) < interval {
                return false;
            }
        }
        guard.insert(task_id.to_string(), now_instant);
    }

    // 2. 持久化节流：跨进程重启也生效
    if let Some(parsed) = last_run_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    {
        let elapsed = now.signed_duration_since(parsed.with_timezone(&Utc));
        if elapsed.num_seconds() >= 0
            && (elapsed.num_seconds() as u64) < min_interval_seconds.max(60)
        {
            return false;
        }
    }
    true
}

/// 在 Gateway dispatch 完成后调用：扫描以本任务为依赖的 chain 任务，
/// 满足 on_status 条件则在新 task 上 dispatch。
pub fn resolve_chain_after_dispatch(
    app: &AppHandle,
    finished_task_id: &str,
    finished_success: bool,
) {
    let app_handle = app.clone();
    let task_id = finished_task_id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_chain_resolver(&app_handle, &task_id, finished_success) {
            tracing::warn!("[Wakeup Chain] resolver 失败: {}", err);
        }
    });
}

fn run_chain_resolver(
    app: &AppHandle,
    finished_task_id: &str,
    finished_success: bool,
) -> Result<(), String> {
    let db = app.state::<Database>();
    let state = WakeupService::load_state(&db)?;
    if !state.enabled {
        return Ok(());
    }

    for task in state.tasks.into_iter() {
        if !task.enabled || task.trigger_mode.trim() != "chain" {
            continue;
        }
        let Some(chain) = task.chain_depends_on.as_ref() else {
            continue;
        };
        if chain.depends_on_task_id != finished_task_id {
            continue;
        }
        let allow = match chain.on_status.trim() {
            "any" => true,
            "failure" => !finished_success,
            _ => finished_success, // success
        };
        if !allow {
            continue;
        }
        if task.account_id.trim().is_empty()
            || task.model.trim().is_empty()
            || task.command_template.trim().is_empty()
        {
            continue;
        }

        let triggered_by = format!(
            "chain:{}({})",
            finished_task_id,
            if finished_success { "success" } else { "failure" }
        );
        let req = DispatchRequest {
            kind: RunKind::Chain,
            task_template: task.clone(),
            accounts: vec![DispatchAccountInput {
                account_id: task.account_id.clone(),
                task_id: Some(task.id.clone()),
                task_name: if task.name.trim().is_empty() {
                    "未命名任务".to_string()
                } else {
                    task.name.clone()
                },
            }],
            triggered_by,
            run_id: None,
            mutate_task: true,
        };
        if let Err(err) = WakeupGateway::dispatch(app, req) {
            tracing::warn!(
                "[Wakeup Chain] 链式派发失败 downstream_task_id={} err={}",
                task.id,
                err
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_blocks_within_min_interval() {
        let id = "chain-test-acct";
        let _ = last_trigger_map().lock().ok().map(|mut g| g.remove(id));
        let now = Utc::now();
        assert!(try_acquire_event_slot(id, 120, &None, now));
        assert!(!try_acquire_event_slot(id, 120, &None, now));
    }

    #[test]
    fn persistent_throttle_uses_last_run_at() {
        let id = "throttle-persist";
        let _ = last_trigger_map().lock().ok().map(|mut g| g.remove(id));
        let now = Utc::now();
        let recent = (now - chrono::Duration::seconds(30)).to_rfc3339();
        // even fresh memory slot, persistent guard should block
        assert!(!try_acquire_event_slot(id, 120, &Some(recent), now));
    }

    #[test]
    fn persistent_throttle_passes_after_window() {
        let id = "throttle-passes";
        let _ = last_trigger_map().lock().ok().map(|mut g| g.remove(id));
        let now = Utc::now();
        let old = (now - chrono::Duration::seconds(600)).to_rfc3339();
        assert!(try_acquire_event_slot(id, 120, &Some(old), now));
    }
}
