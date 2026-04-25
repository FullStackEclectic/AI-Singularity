use super::gateway::{
    dispatch_outcome_to_batch_result, DispatchAccountInput, DispatchRequest, RunKind,
    WakeupGateway,
};
use super::{WakeupService, WakeupTask, WakeupVerificationBatchResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::AppHandle;

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
        let now = chrono::Utc::now().to_rfc3339();
        let task_template = WakeupTask {
            id: String::new(),
            name: "批次验证".to_string(),
            enabled: true,
            account_id: String::new(),
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
            updated_at: now,
            last_run_at: None,
            last_status: None,
            last_category: None,
            last_message: None,
            consecutive_failures: 0,
            event_subscribe: None,
            chain_depends_on: None,
        };

        let accounts: Vec<DispatchAccountInput> = account_ids
            .into_iter()
            .filter(|id| !id.trim().is_empty())
            .map(|account_id| DispatchAccountInput {
                account_id,
                task_id: None,
                task_name: "批次验证".to_string(),
            })
            .collect();

        let req = DispatchRequest {
            kind: RunKind::Verification,
            task_template,
            accounts,
            triggered_by: "verification.batch".to_string(),
            run_id: run_id.map(|v| v.to_string()),
            mutate_task: false,
        };

        let outcome = WakeupGateway::dispatch(app, req)?;
        Ok(dispatch_outcome_to_batch_result(outcome))
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

pub(super) fn register_cancellation_flag(run_id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = cancellation_map().lock() {
        guard.insert(run_id.to_string(), flag.clone());
    }
    flag
}

pub(super) fn unregister_cancellation_flag(run_id: &str) {
    if let Ok(mut guard) = cancellation_map().lock() {
        guard.remove(run_id);
    }
}
