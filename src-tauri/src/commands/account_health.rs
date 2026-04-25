use crate::db::Database;
use crate::error::AppResult;
use crate::models::IdeAccount;
use crate::services::account_health::AccountHealthService;
use crate::services::event_bus::EventBus;
use crate::services::token_keeper::{TokenHealthOverview, TokenKeeper};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn list_disabled_ide_accounts(db: State<'_, Database>) -> AppResult<Vec<IdeAccount>> {
    Ok(AccountHealthService::list_disabled(&db))
}

#[tauri::command]
pub async fn clear_ide_account_disabled(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
) -> AppResult<usize> {
    let rows = db
        .clear_ide_account_disabled(&id)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))?;
    if rows > 0 {
        EventBus::emit_data_changed(
            &app,
            "ide_accounts",
            "clear_disabled",
            "account_health.clear",
        );
    }
    Ok(rows)
}

#[tauri::command]
pub async fn mark_ide_account_disabled(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    reason: String,
) -> AppResult<usize> {
    let rows = db
        .mark_ide_account_disabled(&id, &reason)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))?;
    if rows > 0 {
        EventBus::emit_data_changed(
            &app,
            "ide_accounts",
            "mark_disabled",
            "account_health.mark",
        );
    }
    Ok(rows)
}

#[tauri::command]
pub async fn get_token_health_overview(
    db: State<'_, Database>,
) -> AppResult<TokenHealthOverview> {
    Ok(TokenKeeper::snapshot_overview(&db))
}
