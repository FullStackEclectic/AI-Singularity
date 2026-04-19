use super::{
    WakeupHistoryItem, WakeupService, WakeupTask, SCHEDULER_INTERVAL_SECS,
    WAKEUP_SCHEDULER_STARTED,
};
use super::execution::{
    apply_attempt_outcome_to_task, execute_wakeup_task_with_retry, notify_task_auto_paused,
};
use crate::db::Database;
use crate::services::event_bus::EventBus;
use chrono::{DateTime, Datelike, Local, Utc, Weekday};
use cron::Schedule;
use std::path::Path;
use std::str::FromStr;
use tauri::{AppHandle, Manager};

pub(super) fn parse_rfc3339_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub(super) fn next_task_due_time(
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

pub(super) fn describe_task_trigger(task: &WakeupTask) -> String {
    let version_desc = format!(
        "客户端模式 {}（回退 {}）",
        super::execution::normalize_client_version_mode(&task.client_version_mode),
        super::execution::normalize_client_version_mode(&task.client_version_fallback_mode)
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

impl WakeupService {
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

    pub fn ensure_scheduler_started(app: AppHandle, app_data_dir: std::path::PathBuf) {
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
        let window_start =
            now - chrono::Duration::seconds((SCHEDULER_INTERVAL_SECS as i64) + 35);
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
            let _ = Self::save_state(app_data_dir, state)?;
            if !history_items.is_empty() {
                let _ = Self::add_history_items(app_data_dir, history_items)?;
            }
            EventBus::emit_data_changed(app, "wakeup", "scheduler_tick", "wakeup.scheduler");
        }

        Ok(())
    }
}

fn next_due_in_window(
    cron_expr: &str,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, String) {
    let schedule = match Schedule::from_str(cron_expr) {
        Ok(value) => value,
        Err(err) => return (None, format!("cron 解析失败（{}）：{}", cron_expr, err)),
    };
    let Some(candidate) = schedule.after(&window_start).next() else {
        return (None, format!("cron 在窗口内没有候选时间（{}）", cron_expr));
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
                    format!(
                        "primary/secondary 都未命中窗口或日期策略 {}",
                        describe_window_day_policy(task)
                    ),
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
            let (fallback_due, fallback_reason) =
                fallback_due_for_secondary_window_on_primary_failure(
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
                    format!("primary_window 未命中，且未触发回退。{}", fallback_reason),
                )
            }
        }
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
        return (
            None,
            "secondary_window 不晚于 primary_window，跳过回退".to_string(),
        );
    }
    if !in_window_and_policy(secondary) {
        return (
            None,
            "secondary_window 未命中当前窗口或日期策略".to_string(),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};

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
        let monday = DateTime::<Utc>::from_timestamp(1_735_526_400, 0).expect("valid ts");
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
}
