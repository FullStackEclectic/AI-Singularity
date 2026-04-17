use crate::services::event_bus::EventBus;
use crate::services::wakeup::{
    WakeupHistoryItem, WakeupService, WakeupState, WakeupVerificationBatchResult,
};
use tauri::{AppHandle, Manager};

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))
}

#[tauri::command]
pub fn wakeup_get_state(app: AppHandle) -> Result<WakeupState, String> {
    WakeupService::load_state(&app_data_dir(&app)?)
}

#[tauri::command]
pub fn wakeup_save_state(app: AppHandle, state: WakeupState) -> Result<WakeupState, String> {
    let saved = WakeupService::save_state(&app_data_dir(&app)?, state)?;
    EventBus::emit_data_changed(&app, "wakeup", "save_state", "wakeup.save_state");
    Ok(saved)
}

#[tauri::command]
pub fn wakeup_load_history(app: AppHandle) -> Result<Vec<WakeupHistoryItem>, String> {
    WakeupService::load_history(&app_data_dir(&app)?)
}

#[tauri::command]
pub fn wakeup_add_history(
    app: AppHandle,
    items: Vec<WakeupHistoryItem>,
) -> Result<Vec<WakeupHistoryItem>, String> {
    let history = WakeupService::add_history_items(&app_data_dir(&app)?, items)?;
    EventBus::emit_data_changed(&app, "wakeup", "add_history", "wakeup.add_history");
    Ok(history)
}

#[tauri::command]
pub fn wakeup_clear_history(app: AppHandle) -> Result<(), String> {
    WakeupService::clear_history(&app_data_dir(&app)?)?;
    EventBus::emit_data_changed(&app, "wakeup", "clear_history", "wakeup.clear_history");
    Ok(())
}

#[tauri::command]
pub fn wakeup_run_verification_batch(
    app: AppHandle,
    account_ids: Vec<String>,
    model: String,
    prompt: String,
    command_template: String,
    timeout_seconds: Option<u64>,
    retry_failed_times: Option<u8>,
    run_id: Option<String>,
) -> Result<WakeupVerificationBatchResult, String> {
    WakeupService::run_verification_batch(
        &app,
        account_ids,
        &model,
        &prompt,
        &command_template,
        timeout_seconds.unwrap_or(120).max(10),
        retry_failed_times.unwrap_or(1).min(5) as usize,
        run_id.as_deref(),
    )
}

#[tauri::command]
pub fn wakeup_cancel_verification_run(run_id: String) -> Result<bool, String> {
    WakeupService::cancel_verification_run(&run_id)
}

#[tauri::command]
pub fn wakeup_run_task_now(app: AppHandle, task_id: String) -> Result<WakeupState, String> {
    let _ = WakeupService::run_task_now(&app, &task_id)?;
    WakeupService::load_state(&app_data_dir(&app)?)
}
