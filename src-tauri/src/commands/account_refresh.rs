use crate::db::Database;
use crate::error::AppResult;
use crate::services::account_refresh_orchestrator::{
    AccountRefreshOrchestrator, RefreshStats, RefreshTrigger,
};
use crate::services::event_bus::EventBus;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn refresh_all_ide_accounts(
    app: AppHandle,
    db: State<'_, Database>,
    trigger: Option<String>,
) -> AppResult<RefreshStats> {
    let trig = match trigger.as_deref() {
        Some("auto") => RefreshTrigger::Auto,
        _ => RefreshTrigger::ManualBatch,
    };
    let db_arc = Arc::new(db.inner().clone());
    let stats = AccountRefreshOrchestrator::refresh_all(db_arc, trig).await;
    EventBus::emit_data_changed(
        &app,
        "ide_accounts",
        "refresh_all",
        "account_refresh.batch",
    );
    Ok(stats)
}
