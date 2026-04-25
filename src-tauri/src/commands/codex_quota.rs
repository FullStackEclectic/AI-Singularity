use crate::db::{CodexQuotaCacheStats, Database};
use crate::error::AppResult;
use crate::services::event_bus::EventBus;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn codex_get_quota_cache_stats(
    db: State<'_, Database>,
) -> AppResult<CodexQuotaCacheStats> {
    db.codex_quota_cache_stats()
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))
}

#[tauri::command]
pub async fn codex_clear_quota_cache(
    app: AppHandle,
    db: State<'_, Database>,
    account_id: Option<String>,
) -> AppResult<usize> {
    let removed = db
        .delete_codex_quota_cache(account_id.as_deref())
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))?;
    if removed > 0 {
        EventBus::emit_data_changed(
            &app,
            "codex_quota_cache",
            "cleared",
            "codex_quota.clear_cache",
        );
    }
    Ok(removed)
}
