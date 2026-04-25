use crate::db::{Database, WakeupCategorySummary, WakeupHistoryRow, WakeupRunRow};
use crate::services::event_bus::EventBus;
use crate::services::wakeup::{
    WakeupGateway, WakeupHistoryItem, WakeupService, WakeupState, WakeupVerificationBatchResult,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn wakeup_get_state(app: AppHandle) -> Result<WakeupState, String> {
    let db = app.state::<Database>();
    WakeupService::load_state(&db)
}

#[tauri::command]
pub fn wakeup_save_state(app: AppHandle, state: WakeupState) -> Result<WakeupState, String> {
    let db = app.state::<Database>();
    let saved = WakeupService::save_state(&db, state)?;
    EventBus::emit_data_changed(&app, "wakeup", "save_state", "wakeup.save_state");
    Ok(saved)
}

#[tauri::command]
pub fn wakeup_load_history(app: AppHandle) -> Result<Vec<WakeupHistoryItem>, String> {
    let db = app.state::<Database>();
    WakeupService::load_history(&db)
}

#[tauri::command]
pub fn wakeup_add_history(
    app: AppHandle,
    items: Vec<WakeupHistoryItem>,
) -> Result<Vec<WakeupHistoryItem>, String> {
    let db = app.state::<Database>();
    let history = WakeupService::add_history_items(&db, items)?;
    EventBus::emit_data_changed(&app, "wakeup", "add_history", "wakeup.add_history");
    Ok(history)
}

#[tauri::command]
pub fn wakeup_clear_history(app: AppHandle) -> Result<(), String> {
    let db = app.state::<Database>();
    WakeupService::clear_history(&db)?;
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
    let db = app.state::<Database>();
    WakeupService::load_state(&db)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupRuntimeStatus {
    pub concurrency_in_use: usize,
    pub concurrency_limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupRunsPage {
    pub items: Vec<WakeupRunRow>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupSummary24h {
    pub categories: Vec<WakeupCategorySummary>,
    pub total_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
}

#[tauri::command]
pub fn wakeup_list_runs(
    app: AppHandle,
    kind: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<WakeupRunsPage, String> {
    let db = app.state::<Database>();
    let kind_filter = kind.as_deref().filter(|v| !v.trim().is_empty());
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let offset = offset.unwrap_or(0);
    let items = db
        .list_wakeup_runs(kind_filter, limit, offset)
        .map_err(|e| format!("读取 Wakeup 运行列表失败: {}", e))?;
    let total = db
        .count_wakeup_runs(kind_filter)
        .map_err(|e| format!("统计 Wakeup 运行数失败: {}", e))?;
    Ok(WakeupRunsPage { items, total })
}

#[tauri::command]
pub fn wakeup_get_run_items(
    app: AppHandle,
    run_id: String,
) -> Result<Vec<WakeupHistoryRow>, String> {
    let db = app.state::<Database>();
    db.list_wakeup_history(Some(run_id.as_str()), 1_000)
        .map_err(|e| format!("读取 Wakeup 运行明细失败: {}", e))
}

#[tauri::command]
pub fn wakeup_get_runtime_status(app: AppHandle) -> Result<WakeupRuntimeStatus, String> {
    let db = app.state::<Database>();
    Ok(WakeupRuntimeStatus {
        concurrency_in_use: WakeupGateway::current_in_flight(),
        concurrency_limit: WakeupGateway::concurrency_limit(&db),
    })
}

#[tauri::command]
pub fn wakeup_get_summary_24h(app: AppHandle) -> Result<WakeupSummary24h, String> {
    let db = app.state::<Database>();
    let categories = db
        .list_wakeup_summary_24h()
        .map_err(|e| format!("读取 Wakeup 24h 概览失败: {}", e))?;
    let total_count: i64 = categories.iter().map(|c| c.total).sum();
    let success_count: i64 = categories.iter().map(|c| c.success).sum();
    let failure_count = (total_count - success_count).max(0);
    Ok(WakeupSummary24h {
        categories,
        total_count,
        success_count,
        failure_count,
    })
}
