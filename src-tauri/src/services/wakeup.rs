use crate::db::Database;
use crate::services::event_bus::EventBus;
use crate::services::ide_injector::IdeInjector;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, Utc, Weekday};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const WAKEUP_STATE_FILE: &str = "wakeup_state.json";
const WAKEUP_HISTORY_FILE: &str = "wakeup_history.json";
const MAX_HISTORY_ITEMS: usize = 200;
const SCHEDULER_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupTask {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub account_id: String,
    #[serde(default = "default_trigger_mode")]
    pub trigger_mode: String,
    #[serde(default = "default_reset_window")]
    pub reset_window: String,
    #[serde(default = "default_window_day_policy")]
    pub window_day_policy: String,
    #[serde(default = "default_window_fallback_policy")]
    pub window_fallback_policy: String,
    #[serde(default = "default_client_version_mode")]
    pub client_version_mode: String,
    #[serde(default = "default_client_version_fallback_mode")]
    pub client_version_fallback_mode: String,
    #[serde(default)]
    pub command_template: String,
    pub model: String,
    pub prompt: String,
    pub cron: String,
    pub notes: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub retry_failed_times: u8,
    #[serde(default)]
    pub pause_after_failures: u8,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_category: Option<String>,
    #[serde(default)]
    pub last_message: Option<String>,
    #[serde(default)]
    pub consecutive_failures: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WakeupState {
    pub enabled: bool,
    pub tasks: Vec<WakeupTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupHistoryItem {
    pub id: String,
    #[serde(default)]
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub task_name: String,
    pub account_id: String,
    pub model: String,
    pub status: String,
    #[serde(default = "default_wakeup_category")]
    pub category: String,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupVerificationBatchItem {
    pub account_id: String,
    pub email: String,
    pub status: String,
    pub category: String,
    pub attempts: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupCategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupVerificationBatchResult {
    pub executed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub retried_count: usize,
    pub canceled: bool,
    pub category_counts: Vec<WakeupCategoryCount>,
    pub items: Vec<WakeupVerificationBatchItem>,
}

pub struct WakeupService;
static WAKEUP_SCHEDULER_STARTED: OnceLock<()> = OnceLock::new();
static WAKEUP_CANCELLATION_MAP: OnceLock<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>> =
    OnceLock::new();

fn default_timeout_seconds() -> u64 {
    120
}

fn default_trigger_mode() -> String {
    "cron".to_string()
}

fn default_reset_window() -> String {
    "primary_window".to_string()
}

fn default_window_day_policy() -> String {
    "all_days".to_string()
}

fn default_window_fallback_policy() -> String {
    "none".to_string()
}

fn default_wakeup_category() -> String {
    "unknown".to_string()
}

fn default_client_version_mode() -> String {
    "auto".to_string()
}

fn default_client_version_fallback_mode() -> String {
    "auto".to_string()
}

impl WakeupService {
    pub fn load_state(app_data_dir: &Path) -> Result<WakeupState, String> {
        let path = state_path(app_data_dir);
        if !path.exists() {
            return Ok(WakeupState::default());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("读取 Wakeup 状态失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(WakeupState::default());
        }
        serde_json::from_str(&raw).map_err(|e| format!("解析 Wakeup 状态失败: {}", e))
    }

    pub fn save_state(app_data_dir: &Path, mut state: WakeupState) -> Result<WakeupState, String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let now = Utc::now().to_rfc3339();
        for task in &mut state.tasks {
            if task.created_at.trim().is_empty() {
                task.created_at = now.clone();
            }
            task.client_version_mode = normalize_client_version_mode(&task.client_version_mode);
            task.client_version_fallback_mode =
                normalize_client_version_mode(&task.client_version_fallback_mode);
            task.updated_at = now.clone();
        }
        let content =
            serde_json::to_string_pretty(&state).map_err(|e| format!("序列化 Wakeup 状态失败: {}", e))?;
        fs::write(state_path(app_data_dir), content).map_err(|e| format!("写入 Wakeup 状态失败: {}", e))?;
        Ok(state)
    }

    pub fn load_history(app_data_dir: &Path) -> Result<Vec<WakeupHistoryItem>, String> {
        let path = history_path(app_data_dir);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("读取 Wakeup 历史失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&raw).map_err(|e| format!("解析 Wakeup 历史失败: {}", e))
    }

    pub fn add_history_items(
        app_data_dir: &Path,
        items: Vec<WakeupHistoryItem>,
    ) -> Result<Vec<WakeupHistoryItem>, String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let mut current = Self::load_history(app_data_dir)?;
        current.extend(items);
        current.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        current.truncate(MAX_HISTORY_ITEMS);
        let content = serde_json::to_string_pretty(&current)
            .map_err(|e| format!("序列化 Wakeup 历史失败: {}", e))?;
        fs::write(history_path(app_data_dir), content).map_err(|e| format!("写入 Wakeup 历史失败: {}", e))?;
        Ok(current)
    }

    pub fn clear_history(app_data_dir: &Path) -> Result<(), String> {
        let path = history_path(app_data_dir);
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("清空 Wakeup 历史失败: {}", e))?;
        }
        Ok(())
    }

    pub fn run_task_now(app: &AppHandle, task_id: &str) -> Result<WakeupTask, String> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("获取应用目录失败: {}", e))?;
        let mut state = Self::load_state(&app_data_dir)?;
        let db = app.state::<Database>();
        let now = Utc::now().to_rfc3339();

        let task = state
            .tasks
            .iter_mut()
            .find(|item| item.id == task_id)
            .ok_or_else(|| "未找到对应的 Wakeup 任务".to_string())?;

        if task.account_id.trim().is_empty()
            || task.model.trim().is_empty()
            || task.command_template.trim().is_empty()
        {
            return Err("任务缺少账号、模型或命令模板，无法立即执行".to_string());
        }

        let outcome =
            execute_wakeup_task_with_retry(&db, task, task.retry_failed_times as usize, None);
        let auto_paused = apply_attempt_outcome_to_task(task, &outcome, &now);
        let saved = Self::save_state(&app_data_dir, state)?;
        let updated_task = saved
            .tasks
            .into_iter()
            .find(|item| item.id == task_id)
            .ok_or_else(|| "任务执行后未能重新读取状态".to_string())?;
        let run_id = format!("run-{}", uuid::Uuid::new_v4());

        let _ = Self::add_history_items(
            &app_data_dir,
            vec![WakeupHistoryItem {
                id: format!("history-{}", uuid::Uuid::new_v4()),
                run_id: Some(run_id),
                task_id: Some(updated_task.id.clone()),
                task_name: if updated_task.name.trim().is_empty() {
                    "未命名任务".to_string()
                } else {
                    updated_task.name.clone()
                },
                account_id: updated_task.account_id.clone(),
                model: updated_task.model.clone(),
                status: updated_task
                    .last_status
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                category: outcome.category.clone(),
                message: updated_task.last_message.clone(),
                created_at: now,
            }],
        )?;

        if auto_paused {
            notify_task_auto_paused(app, &updated_task);
        }

        EventBus::emit_data_changed(app, "wakeup", "run_task_now", "wakeup.run_task_now");
        Ok(updated_task)
    }

    pub fn run_verification_batch(
        app: &AppHandle,
        account_ids: Vec<String>,
        model: &str,
        prompt: &str,
        command_template: &str,
        timeout_seconds: u64,
        retry_failed_times: usize,
        run_id: Option<&str>,
    ) -> Result<WakeupVerificationBatchResult, String> {
        let db = app.state::<Database>();
        let accounts = db.get_all_ide_accounts().map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        let mut history_items = Vec::new();
        let mut retried_count = 0usize;
        let now = Utc::now().to_rfc3339();
        let run_id = run_id
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("run-{}", uuid::Uuid::new_v4()));
        let cancel_flag = register_cancellation_flag(&run_id);
        let mut canceled = false;

        for account_id in account_ids.into_iter().filter(|id| !id.trim().is_empty()) {
            if cancel_flag.load(Ordering::Relaxed) {
                canceled = true;
                break;
            }
            let Some(account) = accounts.iter().find(|item| item.id == account_id) else {
                items.push(WakeupVerificationBatchItem {
                    account_id: account_id.clone(),
                    email: "未知账号".to_string(),
                    status: "error".to_string(),
                    category: "account_not_found".to_string(),
                    attempts: 0,
                    message: "未找到对应的 IDE 账号".to_string(),
                });
                continue;
            };

            let task = WakeupTask {
                id: format!("verification-{}", account.id),
                name: "批次验证".to_string(),
                enabled: true,
                account_id: account.id.clone(),
                trigger_mode: "cron".to_string(),
                reset_window: "primary_window".to_string(),
                window_day_policy: "all_days".to_string(),
                window_fallback_policy: "none".to_string(),
                client_version_mode: "auto".to_string(),
                client_version_fallback_mode: "auto".to_string(),
                command_template: command_template.to_string(),
                model: model.to_string(),
                prompt: prompt.to_string(),
                cron: String::new(),
                notes: None,
                timeout_seconds,
                retry_failed_times: retry_failed_times.min(5) as u8,
                pause_after_failures: 0,
                created_at: now.clone(),
                updated_at: now.clone(),
                last_run_at: None,
                last_status: None,
                last_category: None,
                last_message: None,
                consecutive_failures: 0,
            };

            let outcome =
                execute_wakeup_task_with_retry(&db, &task, retry_failed_times, Some(cancel_flag.as_ref()));
            if outcome.execution.message.contains("用户已取消当前批次验证") {
                canceled = true;
            }
            retried_count += outcome.attempts.saturating_sub(1);
            let status = if outcome.execution.success {
                "success"
            } else {
                "error"
            }
            .to_string();

            items.push(WakeupVerificationBatchItem {
                account_id: account.id.clone(),
                email: account.email.clone(),
                status: status.clone(),
                category: outcome.category.clone(),
                attempts: outcome.attempts,
                message: outcome.execution.message.clone(),
            });

            history_items.push(WakeupHistoryItem {
                id: format!("history-{}", uuid::Uuid::new_v4()),
                run_id: Some(run_id.clone()),
                task_id: None,
                task_name: "批次验证".to_string(),
                account_id: account.id.clone(),
                model: model.to_string(),
                status,
                category: outcome.category.clone(),
                message: Some(if outcome.attempts > 1 {
                    format!("（尝试 {} 次）{}", outcome.attempts, outcome.execution.message)
                } else {
                    outcome.execution.message
                }),
                created_at: Utc::now().to_rfc3339(),
            });

            if canceled {
                break;
            }
        }

        if !history_items.is_empty() {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("获取应用目录失败: {}", e))?;
            let _ = Self::add_history_items(&app_data_dir, history_items)?;
            EventBus::emit_data_changed(app, "wakeup", "verification_batch", "wakeup.verification_batch");
        }

        let success_count = items.iter().filter(|item| item.status == "success").count();
        let failed_count = items.len().saturating_sub(success_count);
        let mut category_map = std::collections::BTreeMap::<String, usize>::new();
        for item in &items {
            *category_map.entry(item.category.clone()).or_insert(0) += 1;
        }
        let category_counts = category_map
            .into_iter()
            .map(|(category, count)| WakeupCategoryCount { category, count })
            .collect::<Vec<_>>();
        unregister_cancellation_flag(&run_id);

        Ok(WakeupVerificationBatchResult {
            executed_count: items.len(),
            success_count,
            failed_count,
            retried_count,
            canceled,
            category_counts,
            items,
        })
    }

    pub fn cancel_verification_run(run_id: &str) -> Result<bool, String> {
        let map = cancellation_map();
        let guard = map
            .lock()
            .map_err(|_| "无法获取取消任务锁".to_string())?;
        let Some(flag) = guard.get(run_id) else {
            return Ok(false);
        };
        flag.store(true, Ordering::Relaxed);
        Ok(true)
    }

    pub fn ensure_scheduler_started(app: AppHandle, app_data_dir: PathBuf) {
        if WAKEUP_SCHEDULER_STARTED.set(()).is_err() {
            return;
        }

        tauri::async_runtime::spawn(async move {
            let app_handle = app.clone();
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(SCHEDULER_INTERVAL_SECS)).await;
                if let Err(err) = Self::run_scheduler_tick(&app_handle, &app_data_dir) {
                    tracing::warn!("[Wakeup] 调度器执行失败: {}", err);
                }
            }
        });
    }

    fn run_scheduler_tick(app: &AppHandle, app_data_dir: &Path) -> Result<(), String> {
        let mut state = Self::load_state(app_data_dir)?;
        if !state.enabled {
            return Ok(());
        }
        let db = app.state::<Database>();

        let now = Utc::now();
        let window_start = now - ChronoDuration::seconds((SCHEDULER_INTERVAL_SECS as i64) + 35);
        let mut history_items = Vec::new();
        let mut changed = false;

        for task in &mut state.tasks {
            if !task.enabled {
                continue;
            }
            if task.account_id.trim().is_empty()
                || task.model.trim().is_empty()
                || task.command_template.trim().is_empty()
            {
                continue;
            }

            let (scheduled_for, schedule_reason) = next_task_due_time(&db, task, window_start, now);
            let Some(scheduled_for) = scheduled_for else {
                tracing::debug!(
                    "[Wakeup] 任务跳过: id={} name={} reason={}",
                    task.id,
                    if task.name.trim().is_empty() {
                        "未命名任务"
                    } else {
                        task.name.as_str()
                    },
                    schedule_reason
                );
                continue;
            };
            tracing::debug!(
                "[Wakeup] 任务命中: id={} name={} scheduled_for={} reason={}",
                task.id,
                if task.name.trim().is_empty() {
                    "未命名任务"
                } else {
                    task.name.as_str()
                },
                scheduled_for.to_rfc3339(),
                schedule_reason
            );

            let already_ran = task
                .last_run_at
                .as_deref()
                .and_then(parse_rfc3339_utc)
                .is_some_and(|last| last >= scheduled_for);
            if already_ran {
                continue;
            }

            let outcome =
                execute_wakeup_task_with_retry(&db, task, task.retry_failed_times as usize, None);
            let auto_paused = apply_attempt_outcome_to_task(task, &outcome, &now.to_rfc3339());
            changed = true;
            let run_id = format!("run-{}", uuid::Uuid::new_v4());

            history_items.push(WakeupHistoryItem {
                id: format!("history-{}", uuid::Uuid::new_v4()),
                run_id: Some(run_id),
                task_id: Some(task.id.clone()),
                task_name: if task.name.trim().is_empty() {
                    "未命名任务".to_string()
                } else {
                    task.name.clone()
                },
                account_id: task.account_id.clone(),
                model: task.model.clone(),
                status: if outcome.execution.success {
                    "success".to_string()
                } else {
                    "error".to_string()
                },
                category: outcome.category.clone(),
                message: Some(format!(
                    "调度器命中了 {}（{}），执行了 {} 次。{}",
                    describe_task_trigger(task),
                    schedule_reason,
                    outcome.attempts,
                    outcome.execution.message
                )),
                created_at: now.to_rfc3339(),
            });

            if auto_paused {
                notify_task_auto_paused(app, task);
            }
        }

        if changed {
            let saved = Self::save_state(app_data_dir, state)?;
            let _ = saved;
            if !history_items.is_empty() {
                let _ = Self::add_history_items(app_data_dir, history_items)?;
            }
            EventBus::emit_data_changed(app, "wakeup", "scheduler_tick", "wakeup.scheduler");
        }

        Ok(())
    }
}

struct WakeupAttemptOutcome {
    execution: WakeupExecutionResult,
    attempts: usize,
    category: String,
}

fn execute_wakeup_task_with_retry(
    db: &Database,
    task: &WakeupTask,
    retry_failed_times: usize,
    cancel_flag: Option<&AtomicBool>,
) -> WakeupAttemptOutcome {
    let mut execution = execute_wakeup_task(db, task, cancel_flag);
    let mut attempts = 1usize;
    while !execution.success && attempts <= retry_failed_times.min(5) {
        if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            break;
        }
        attempts += 1;
        execution = execute_wakeup_task(db, task, cancel_flag);
    }
    let category = classify_execution_category(execution.success, &execution.message);
    WakeupAttemptOutcome {
        execution,
        attempts,
        category,
    }
}

fn apply_attempt_outcome_to_task(
    task: &mut WakeupTask,
    outcome: &WakeupAttemptOutcome,
    run_at: &str,
) -> bool {
    task.last_run_at = Some(run_at.to_string());
    task.updated_at = run_at.to_string();
    task.last_status = Some(if outcome.execution.success {
        "success".to_string()
    } else {
        "error".to_string()
    });
    task.last_category = Some(outcome.category.clone());

    if outcome.execution.success {
        task.consecutive_failures = 0;
        task.last_message = Some(if outcome.attempts > 1 {
            format!("执行成功（共尝试 {} 次）。{}", outcome.attempts, outcome.execution.message)
        } else {
            outcome.execution.message.clone()
        });
        return false;
    }

    task.consecutive_failures = task.consecutive_failures.saturating_add(1);
    let mut message = if outcome.attempts > 1 {
        format!("执行失败（共尝试 {} 次）。{}", outcome.attempts, outcome.execution.message)
    } else {
        outcome.execution.message.clone()
    };
    let should_pause = task.pause_after_failures > 0
        && task.consecutive_failures >= task.pause_after_failures;
    if should_pause {
        task.enabled = false;
        message.push_str(&format!(
            " 已连续失败 {} 次，任务已自动暂停。",
            task.consecutive_failures
        ));
    }
    task.last_message = Some(message);
    should_pause
}

fn notify_task_auto_paused(app: &AppHandle, task: &WakeupTask) {
    let title = format!(
        "Wakeup 任务已自动暂停: {}",
        if task.name.trim().is_empty() {
            "未命名任务"
        } else {
            task.name.as_str()
        }
    );
    let body = task
        .last_message
        .clone()
        .unwrap_or_else(|| "任务连续失败，已自动暂停。".to_string());
    let _ = app.notification().builder().title(&title).body(&body).show();
}

fn parse_rfc3339_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn next_due_in_window(
    cron_expr: &str,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, String) {
    let schedule = match Schedule::from_str(cron_expr) {
        Ok(value) => value,
        Err(err) => {
            return (
                None,
                format!("cron 解析失败（{}）：{}", cron_expr, err),
            )
        }
    };
    let Some(candidate) = schedule.after(&window_start).next() else {
        return (
            None,
            format!("cron 在窗口内没有候选时间（{}）", cron_expr),
        );
    };
    if candidate <= now {
        (
            Some(candidate),
            format!("cron 命中窗口，候选时间 {}", candidate.to_rfc3339()),
        )
    } else {
        (
            None,
            format!(
                "cron 下次触发 {} 仍晚于窗口结束 {}",
                candidate.to_rfc3339(),
                now.to_rfc3339()
            ),
        )
    }
}

fn next_task_due_time(
    db: &Database,
    task: &WakeupTask,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, String) {
    match task.trigger_mode.trim() {
        "quota_reset" => next_due_for_quota_reset(db, task, window_start, now),
        _ => {
            if task.cron.trim().is_empty() {
                (None, "cron 为空，无法命中".to_string())
            } else {
                next_due_in_window(&task.cron, window_start, now)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedWakeupClientProfile {
    requested_mode: String,
    fallback_mode: String,
    effective_mode: String,
    runtime_args: String,
    gateway_mode: String,
    gateway_transport: String,
    gateway_routing: String,
    gateway_version_hint: String,
    fallback_reason: Option<String>,
}

fn normalize_client_version_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "official_stable" | "stable" => "official_stable".to_string(),
        "official_preview" | "preview" | "beta" => "official_preview".to_string(),
        "official_legacy" | "legacy" | "v1_legacy" => "official_legacy".to_string(),
        _ => "auto".to_string(),
    }
}

fn platform_client_family(origin_platform: &str) -> &'static str {
    let platform = origin_platform.trim().to_ascii_lowercase();
    if platform.contains("gemini") {
        "gemini"
    } else if platform.contains("codex") {
        "codex"
    } else {
        "generic"
    }
}

fn is_mode_supported_for_family(family: &str, mode: &str) -> bool {
    if mode == "auto" {
        return true;
    }
    match family {
        "gemini" | "codex" => matches!(mode, "official_stable" | "official_preview" | "official_legacy"),
        _ => mode == "official_legacy",
    }
}

fn profile_fields_for_mode(
    family: &str,
    mode: &str,
) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    match (family, mode) {
        ("gemini", "official_stable") => (
            "--client-channel stable",
            "strict",
            "oauth_refresh",
            "gemini_official",
            "Gemini 官方稳定通道",
        ),
        ("gemini", "official_preview") => (
            "--client-channel preview --enable-preview",
            "compat_preview",
            "oauth_refresh",
            "gemini_preview",
            "Gemini 官方预览通道",
        ),
        ("gemini", "official_legacy") => (
            "--legacy-auth-flow",
            "legacy_compat",
            "oauth_legacy",
            "gemini_legacy",
            "Gemini 旧版兼容链路",
        ),
        ("codex", "official_stable") => (
            "--channel stable",
            "strict",
            "oauth_token",
            "codex_official",
            "Codex 官方稳定通道",
        ),
        ("codex", "official_preview") => (
            "--channel preview --enable-beta",
            "compat_preview",
            "oauth_token",
            "codex_preview",
            "Codex 官方预览通道",
        ),
        ("codex", "official_legacy") => (
            "--legacy-auth-flow",
            "legacy_compat",
            "oauth_legacy",
            "codex_legacy",
            "Codex 旧版兼容链路",
        ),
        (_, "official_legacy") => (
            "--legacy-auth-flow",
            "legacy_compat",
            "oauth_legacy",
            "generic_legacy",
            "通用旧版兼容链路",
        ),
        _ => (
            "",
            "auto",
            "auto",
            "auto",
            "自动跟随当前官方客户端",
        ),
    }
}

fn resolve_task_client_profile(task: &WakeupTask, origin_platform: &str) -> ResolvedWakeupClientProfile {
    let requested_mode = normalize_client_version_mode(&task.client_version_mode);
    let fallback_mode = normalize_client_version_mode(&task.client_version_fallback_mode);
    let family = platform_client_family(origin_platform);

    let (effective_mode, fallback_reason) = if is_mode_supported_for_family(family, &requested_mode) {
        (requested_mode.clone(), None)
    } else if is_mode_supported_for_family(family, &fallback_mode) {
        (
            fallback_mode.clone(),
            Some(format!(
                "平台 {} 不支持 {}，已回退到 {}",
                origin_platform, requested_mode, fallback_mode
            )),
        )
    } else {
        (
            "auto".to_string(),
            Some(format!(
                "平台 {} 不支持 {} / {}，已强制回退到 auto",
                origin_platform, requested_mode, fallback_mode
            )),
        )
    };

    let (runtime_args, gateway_mode, gateway_transport, gateway_routing, gateway_version_hint) =
        profile_fields_for_mode(family, &effective_mode);

    ResolvedWakeupClientProfile {
        requested_mode,
        fallback_mode,
        effective_mode,
        runtime_args: runtime_args.to_string(),
        gateway_mode: gateway_mode.to_string(),
        gateway_transport: gateway_transport.to_string(),
        gateway_routing: gateway_routing.to_string(),
        gateway_version_hint: gateway_version_hint.to_string(),
        fallback_reason,
    }
}

fn next_due_for_quota_reset(
    db: &Database,
    task: &WakeupTask,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, String) {
    let account = db
        .get_all_ide_accounts()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.id == task.account_id));
    let Some(account) = account else {
        return (None, "未找到任务绑定账号".to_string());
    };
    let Some(quota_json) = account.quota_json else {
        return (None, "账号缺少 quota_json，无法读取重置窗口".to_string());
    };
    let value = match serde_json::from_str::<serde_json::Value>(&quota_json) {
        Ok(item) => item,
        Err(err) => {
            return (None, format!("quota_json 解析失败: {}", err));
        }
    };

    let pick_reset = |keys: &[&str]| -> Option<DateTime<Utc>> {
        keys.iter()
            .find_map(|key| value.get(*key).and_then(|item| item.as_i64()))
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
    };

    let primary = pick_reset(&["hourly_reset_time", "primary_reset_time"]);
    let secondary = pick_reset(&["weekly_reset_time", "secondary_reset_time"]);
    let in_window_and_policy = |time: DateTime<Utc>| {
        time >= window_start && time <= now && matches_window_day_policy(task, time)
    };
    match task.reset_window.trim() {
        "secondary_window" => {
            let Some(candidate) = secondary else {
                return (None, "缺少 secondary_window 时间".to_string());
            };
            if in_window_and_policy(candidate) {
                (
                    Some(candidate),
                    format!(
                        "命中 secondary_window（{}，策略 {}）",
                        candidate.to_rfc3339(),
                        describe_window_day_policy(task)
                    ),
                )
            } else {
                (
                    None,
                    format!(
                        "secondary_window {} 未命中窗口或日期策略 {}",
                        candidate.to_rfc3339(),
                        describe_window_day_policy(task)
                    ),
                )
            }
        }
        "either_window" => {
            let candidate = [primary, secondary]
                .into_iter()
                .flatten()
                .filter(|time| in_window_and_policy(*time))
                .max();
            if let Some(candidate) = candidate {
                (
                    Some(candidate),
                    format!(
                        "命中 either_window（{}，策略 {}）",
                        candidate.to_rfc3339(),
                        describe_window_day_policy(task)
                    ),
                )
            } else {
                (
                    None,
                    format!("primary/secondary 都未命中窗口或日期策略 {}", describe_window_day_policy(task)),
                )
            }
        }
        _ => {
            if let Some(primary_due) = primary.filter(|time| in_window_and_policy(*time)) {
                return (
                    Some(primary_due),
                    format!(
                        "命中 primary_window（{}，策略 {}）",
                        primary_due.to_rfc3339(),
                        describe_window_day_policy(task)
                    ),
                );
            }
            let (fallback_due, fallback_reason) = fallback_due_for_secondary_window_on_primary_failure(
                task,
                primary,
                secondary,
                &in_window_and_policy,
            );
            if let Some(candidate) = fallback_due {
                (
                    Some(candidate),
                    format!(
                        "primary 未命中，已按回退策略命中 secondary_window（{}）",
                        candidate.to_rfc3339()
                    ),
                )
            } else {
                (
                    None,
                    format!(
                        "primary_window 未命中，且未触发回退。{}",
                        fallback_reason
                    ),
                )
            }
        }
    }
}

fn describe_task_trigger(task: &WakeupTask) -> String {
    let version_desc = format!(
        "客户端模式 {}（回退 {}）",
        normalize_client_version_mode(&task.client_version_mode),
        normalize_client_version_mode(&task.client_version_fallback_mode)
    );
    if task.trigger_mode.trim() == "quota_reset" {
        let window = match task.reset_window.trim() {
            "secondary_window" => "quota secondary_window 重置窗口",
            "either_window" => "quota 任一重置窗口",
            _ => "quota primary_window 重置窗口",
        };
        let mut desc = format!("{}（{}）", window, describe_window_day_policy(task));
        if task.reset_window.trim() == "primary_window"
            && task.window_fallback_policy.trim() == "primary_then_secondary_on_failure"
        {
            desc.push_str("，主窗口失败后回退次窗口");
        }
        format!("{}；{}", desc, version_desc)
    } else {
        format!("cron 表达式（{}）；{}", task.cron, version_desc)
    }
}

fn fallback_due_for_secondary_window_on_primary_failure(
    task: &WakeupTask,
    primary: Option<DateTime<Utc>>,
    secondary: Option<DateTime<Utc>>,
    in_window_and_policy: &dyn Fn(DateTime<Utc>) -> bool,
) -> (Option<DateTime<Utc>>, String) {
    if task.window_fallback_policy.trim() != "primary_then_secondary_on_failure" {
        return (None, "回退策略为 none".to_string());
    }
    if task.last_status.as_deref() != Some("error") {
        return (None, "最近执行不是失败，不触发回退".to_string());
    }
    let Some(primary) = primary else {
        return (None, "缺少 primary_window 时间，无法判定回退".to_string());
    };
    let Some(secondary) = secondary else {
        return (None, "缺少 secondary_window 时间，无法回退".to_string());
    };
    if secondary <= primary {
        return (None, "secondary_window 不晚于 primary_window，跳过回退".to_string());
    }
    if !in_window_and_policy(secondary) {
        return (None, "secondary_window 未命中当前窗口或日期策略".to_string());
    }
    let Some(last_run_at) = task.last_run_at.as_deref().and_then(parse_rfc3339_utc) else {
        return (None, "缺少 last_run_at，无法判定是否主窗口失败".to_string());
    };
    if last_run_at < primary || last_run_at >= secondary {
        return (
            None,
            format!(
                "last_run_at={} 不在 [primary={}, secondary={}) 区间",
                last_run_at.to_rfc3339(),
                primary.to_rfc3339(),
                secondary.to_rfc3339()
            ),
        );
    }
    (
        Some(secondary),
        format!(
            "命中回退：主窗口失败后，回退到 secondary_window {}",
            secondary.to_rfc3339()
        ),
    )
}

fn matches_window_day_policy(task: &WakeupTask, candidate: DateTime<Utc>) -> bool {
    match task.window_day_policy.trim() {
        "workdays" => is_workday(candidate),
        "weekends" => is_weekend(candidate),
        _ => true,
    }
}

fn describe_window_day_policy(task: &WakeupTask) -> &'static str {
    match task.window_day_policy.trim() {
        "workdays" => "仅工作日",
        "weekends" => "仅周末",
        _ => "任意日",
    }
}

fn is_workday(candidate: DateTime<Utc>) -> bool {
    matches!(
        candidate.with_timezone(&Local).weekday(),
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
    )
}

fn is_weekend(candidate: DateTime<Utc>) -> bool {
    matches!(
        candidate.with_timezone(&Local).weekday(),
        Weekday::Sat | Weekday::Sun
    )
}

struct WakeupExecutionResult {
    success: bool,
    message: String,
}

fn execute_wakeup_task(
    db: &Database,
    task: &WakeupTask,
    cancel_flag: Option<&AtomicBool>,
) -> WakeupExecutionResult {
    let account = match db
        .get_all_ide_accounts()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.id == task.account_id))
    {
        Some(account) => account,
        None => {
            return WakeupExecutionResult {
                success: false,
                message: "未找到绑定的 IDE 账号。".to_string(),
            }
        }
    };

    if let Err(err) = IdeInjector::execute_injection(&account) {
        return WakeupExecutionResult {
            success: false,
            message: format!("账号注入失败: {}", err),
        };
    }

    let profile = resolve_task_client_profile(task, &account.origin_platform);
    let command = render_command_template(task, &account.email, &profile);
    let profile_desc = if let Some(reason) = &profile.fallback_reason {
        format!(
            "；客户端模式 {}（请求 {}，{}）",
            profile.effective_mode, profile.requested_mode, reason
        )
    } else {
        format!("；客户端模式 {}", profile.effective_mode)
    };
    match run_shell_command(&command, task.timeout_seconds, cancel_flag) {
        Ok(output) => WakeupExecutionResult {
            success: true,
            message: format!(
                "已执行命令：{}{}{}",
                command,
                output
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("；输出：{}", value))
                    .unwrap_or_default(),
                profile_desc,
            ),
        },
        Err(err) => WakeupExecutionResult {
            success: false,
            message: format!("执行命令失败：{}；命令：{}{}", err, command, profile_desc),
        },
    }
}

fn render_command_template(
    task: &WakeupTask,
    email: &str,
    profile: &ResolvedWakeupClientProfile,
) -> String {
    let had_runtime_placeholder = task.command_template.contains("{client_runtime_args}");
    let rendered = task.command_template
        .replace("{model}", &task.model)
        .replace("{prompt}", &task.prompt)
        .replace("{account_id}", &task.account_id)
        .replace("{email}", email)
        .replace("{client_version_mode}", &profile.effective_mode)
        .replace("{client_version_mode_requested}", &profile.requested_mode)
        .replace("{client_version_fallback_mode}", &profile.fallback_mode)
        .replace("{client_runtime_args}", &profile.runtime_args)
        .replace("{gateway_mode}", &profile.gateway_mode)
        .replace("{gateway_transport}", &profile.gateway_transport)
        .replace("{gateway_routing}", &profile.gateway_routing)
        .replace("{gateway_version_hint}", &profile.gateway_version_hint);

    if !had_runtime_placeholder && !profile.runtime_args.trim().is_empty() {
        format!("{} {}", rendered.trim_end(), profile.runtime_args)
    } else {
        rendered
    }
}

fn run_shell_command(
    command: &str,
    timeout_seconds: u64,
    cancel_flag: Option<&AtomicBool>,
) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("cmd")
            .args(["/C", command])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动命令失败: {}", e))?;

        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|e| format!("轮询命令状态失败: {}", e))? {
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("读取命令输出失败: {}", e))?;
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !status.success() {
                    return Err(if stderr.is_empty() {
                        format!("退出码 {:?}", status.code())
                    } else {
                        stderr
                    });
                }
                return Ok(if stdout.is_empty() { None } else { Some(truncate_output(&stdout)) });
            }

            if started.elapsed().as_secs() >= timeout_seconds {
                let _ = child.kill();
                return Err(format!("命令执行超时（{} 秒）", timeout_seconds));
            }

            if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                let _ = child.kill();
                return Err("用户已取消当前批次验证".to_string());
            }

            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut child = Command::new("sh")
            .args(["-lc", command])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动命令失败: {}", e))?;

        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|e| format!("轮询命令状态失败: {}", e))? {
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("读取命令输出失败: {}", e))?;
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !status.success() {
                    return Err(if stderr.is_empty() {
                        format!("退出码 {:?}", status.code())
                    } else {
                        stderr
                    });
                }
                return Ok(if stdout.is_empty() { None } else { Some(truncate_output(&stdout)) });
            }

            if started.elapsed().as_secs() >= timeout_seconds {
                let _ = child.kill();
                return Err(format!("命令执行超时（{} 秒）", timeout_seconds));
            }

            if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                let _ = child.kill();
                return Err("用户已取消当前批次验证".to_string());
            }

            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
}

fn truncate_output(raw: &str) -> String {
    const MAX_CHARS: usize = 280;
    if raw.chars().count() <= MAX_CHARS {
        return raw.to_string();
    }
    raw.chars().take(MAX_CHARS).collect::<String>() + "..."
}

fn classify_execution_category(success: bool, message: &str) -> String {
    if success {
        return "success".to_string();
    }
    let lower = message.to_ascii_lowercase();
    if message.contains("未找到绑定的 IDE 账号") || message.contains("未找到对应的 IDE 账号") {
        return "account_not_found".to_string();
    }
    if message.contains("账号注入失败") {
        return "inject_failed".to_string();
    }
    if message.contains("超时") {
        return "timeout".to_string();
    }
    if message.contains("用户已取消当前批次验证") {
        return "canceled".to_string();
    }
    if lower.contains("not recognized")
        || lower.contains("not found")
        || lower.contains("no such file")
        || message.contains("不是内部或外部命令")
    {
        return "command_not_found".to_string();
    }
    if lower.contains("permission denied") || message.contains("拒绝访问") {
        return "permission_denied".to_string();
    }
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
    {
        return "auth_failed".to_string();
    }
    if lower.contains("429") || lower.contains("rate limit") {
        return "rate_limited".to_string();
    }
    if message.contains("执行命令失败") {
        return "command_failed".to_string();
    }
    if message.contains("缺少账号")
        || message.contains("缺少命令模板")
        || message.contains("缺少模型")
        || message.contains("缺少 cron")
    {
        return "validation_failed".to_string();
    }
    "error_unknown".to_string()
}

fn state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(WAKEUP_STATE_FILE)
}

fn history_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(WAKEUP_HISTORY_FILE)
}

fn cancellation_map() -> &'static Mutex<std::collections::HashMap<String, Arc<AtomicBool>>> {
    WAKEUP_CANCELLATION_MAP.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn register_cancellation_flag(run_id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = cancellation_map().lock() {
        guard.insert(run_id.to_string(), flag.clone());
    }
    flag
}

fn unregister_cancellation_flag(run_id: &str) {
    if let Ok(mut guard) = cancellation_map().lock() {
        guard.remove(run_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> WakeupTask {
        WakeupTask {
            id: "task-1".to_string(),
            name: "test".to_string(),
            enabled: true,
            account_id: "acc-1".to_string(),
            trigger_mode: "quota_reset".to_string(),
            reset_window: "primary_window".to_string(),
            window_day_policy: "all_days".to_string(),
            window_fallback_policy: "none".to_string(),
            client_version_mode: "auto".to_string(),
            client_version_fallback_mode: "auto".to_string(),
            command_template: "echo test".to_string(),
            model: "gpt-5".to_string(),
            prompt: "hi".to_string(),
            cron: "0 */6 * * *".to_string(),
            notes: None,
            timeout_seconds: 120,
            retry_failed_times: 0,
            pause_after_failures: 0,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            last_run_at: None,
            last_status: None,
            last_category: None,
            last_message: None,
            consecutive_failures: 0,
        }
    }

    #[test]
    fn window_day_policy_matches_weekday() {
        let monday = DateTime::<Utc>::from_timestamp(1_735_526_400, 0).expect("valid ts"); // 2024-12-30
        let mut task = sample_task();
        task.window_day_policy = "workdays".to_string();
        assert!(matches_window_day_policy(&task, monday));
        task.window_day_policy = "weekends".to_string();
        assert!(!matches_window_day_policy(&task, monday));
    }

    #[test]
    fn fallback_to_secondary_after_primary_failure() {
        let primary = DateTime::<Utc>::from_timestamp(1_735_700_000, 0).expect("valid ts");
        let secondary = DateTime::<Utc>::from_timestamp(1_735_800_000, 0).expect("valid ts");
        let mut task = sample_task();
        task.window_fallback_policy = "primary_then_secondary_on_failure".to_string();
        task.last_status = Some("error".to_string());
        task.last_run_at = Some((primary + ChronoDuration::minutes(5)).to_rfc3339());

        let (due, reason) = fallback_due_for_secondary_window_on_primary_failure(
            &task,
            Some(primary),
            Some(secondary),
            &|_| true,
        );
        assert_eq!(due, Some(secondary));
        assert!(reason.contains("命中回退"));
    }

    #[test]
    fn no_fallback_when_last_run_is_not_failure() {
        let primary = DateTime::<Utc>::from_timestamp(1_735_700_000, 0).expect("valid ts");
        let secondary = DateTime::<Utc>::from_timestamp(1_735_800_000, 0).expect("valid ts");
        let mut task = sample_task();
        task.window_fallback_policy = "primary_then_secondary_on_failure".to_string();
        task.last_status = Some("success".to_string());
        task.last_run_at = Some((primary + ChronoDuration::minutes(5)).to_rfc3339());

        let (due, reason) = fallback_due_for_secondary_window_on_primary_failure(
            &task,
            Some(primary),
            Some(secondary),
            &|_| true,
        );
        assert_eq!(due, None);
        assert!(reason.contains("最近执行不是失败"));
    }

    #[test]
    fn client_profile_falls_back_for_unsupported_platform() {
        let mut task = sample_task();
        task.client_version_mode = "official_preview".to_string();
        task.client_version_fallback_mode = "official_legacy".to_string();
        let profile = resolve_task_client_profile(&task, "cursor");
        assert_eq!(profile.requested_mode, "official_preview");
        assert_eq!(profile.effective_mode, "official_legacy");
        assert!(profile
            .fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("已回退"));
        assert!(profile.runtime_args.contains("--legacy-auth-flow"));
    }

    #[test]
    fn render_template_appends_runtime_args_when_placeholder_missing() {
        let mut task = sample_task();
        task.command_template = "gemini -m \"{model}\" -p \"{prompt}\"".to_string();
        task.client_version_mode = "official_preview".to_string();
        let profile = resolve_task_client_profile(&task, "gemini");
        let rendered = render_command_template(&task, "demo@example.com", &profile);
        assert!(rendered.contains("gemini -m \"gpt-5\" -p \"hi\""));
        assert!(rendered.contains("--client-channel preview --enable-preview"));
    }

    #[test]
    fn render_template_resolves_gateway_placeholders() {
        let mut task = sample_task();
        task.command_template =
            "cmd --mode {gateway_mode} --transport {gateway_transport} --routing {gateway_routing}"
                .to_string();
        task.client_version_mode = "official_stable".to_string();
        let profile = resolve_task_client_profile(&task, "codex");
        let rendered = render_command_template(&task, "demo@example.com", &profile);
        assert!(rendered.contains("--mode strict"));
        assert!(rendered.contains("--transport oauth_token"));
        assert!(rendered.contains("--routing codex_official"));
    }
}
