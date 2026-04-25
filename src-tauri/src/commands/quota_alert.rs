use crate::db::Database;
use crate::error::AppResult;
use crate::services::quota_alert::{QuotaAlertPayload, QuotaAlertService, QuotaAlertSettings};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_quota_alert_settings(
    db: State<'_, Database>,
) -> AppResult<QuotaAlertSettings> {
    Ok(QuotaAlertService::load_settings(&db))
}

#[tauri::command]
pub async fn set_quota_alert_settings(
    db: State<'_, Database>,
    settings: QuotaAlertSettings,
) -> AppResult<QuotaAlertSettings> {
    QuotaAlertService::save_settings(&db, &settings)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
    Ok(QuotaAlertService::load_settings(&db))
}

#[tauri::command]
pub async fn preview_quota_alerts(
    app: AppHandle,
    db: State<'_, Database>,
) -> AppResult<Vec<QuotaAlertPayload>> {
    Ok(QuotaAlertService::run_if_needed(&db, Some(&app)))
}
