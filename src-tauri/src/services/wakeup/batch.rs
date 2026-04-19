use super::{
    WakeupCategoryCount, WakeupHistoryItem, WakeupService, WakeupTask,
    WakeupVerificationBatchItem, WakeupVerificationBatchResult,
};
use super::execution::execute_wakeup_task_with_retry;
use crate::services::event_bus::EventBus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Manager};

static WAKEUP_CANCELLATION_MAP: OnceLock<
    Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
> = OnceLock::new();

impl WakeupService {
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
        let db = app.state::<crate::db::Database>();
        let accounts = db.get_all_ide_accounts().map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        let mut history_items = Vec::new();
        let mut retried_count = 0usize;
        let now = chrono::Utc::now().to_rfc3339();
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

            let outcome = execute_wakeup_task_with_retry(
                &db,
                &task,
                retry_failed_times,
                Some(cancel_flag.as_ref()),
            );
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
                    format!(
                        "（尝试 {} 次）{}",
                        outcome.attempts, outcome.execution.message
                    )
                } else {
                    outcome.execution.message
                }),
                created_at: chrono::Utc::now().to_rfc3339(),
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
            EventBus::emit_data_changed(
                app,
                "wakeup",
                "verification_batch",
                "wakeup.verification_batch",
            );
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
        let guard = map.lock().map_err(|_| "无法获取取消任务锁".to_string())?;
        let Some(flag) = guard.get(run_id) else {
            return Ok(false);
        };
        flag.store(true, Ordering::Relaxed);
        Ok(true)
    }
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
