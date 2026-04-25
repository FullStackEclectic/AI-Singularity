use crate::db::{AccountSwitchHistoryItem, Database};
use crate::error::AppResult;
use crate::services::account_auto_switch::{
    AutoSwitchGroupDefinition, AutoSwitchOutcome, AutoSwitchService, AutoSwitchSettings,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_auto_switch_settings(db: State<'_, Database>) -> AppResult<AutoSwitchSettings> {
    Ok(AutoSwitchService::load_settings(&db))
}

#[tauri::command]
pub async fn set_auto_switch_settings(
    db: State<'_, Database>,
    settings: AutoSwitchSettings,
) -> AppResult<AutoSwitchSettings> {
    AutoSwitchService::save_settings(&db, &settings)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
    Ok(AutoSwitchService::load_settings(&db))
}

#[tauri::command]
pub async fn list_auto_switch_groups() -> AppResult<Vec<AutoSwitchGroupDefinition>> {
    Ok(AutoSwitchService::default_groups())
}

#[tauri::command]
pub async fn run_auto_switch_now(
    app: AppHandle,
    db: State<'_, Database>,
) -> AppResult<AutoSwitchOutcome> {
    AutoSwitchService::run_if_needed(&db, Some(&app))
        .await
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))
}

#[tauri::command]
pub async fn list_account_switch_history(
    db: State<'_, Database>,
    limit: Option<u32>,
) -> AppResult<Vec<AccountSwitchHistoryItem>> {
    db.list_account_switch_history(limit.unwrap_or(50))
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))
}
