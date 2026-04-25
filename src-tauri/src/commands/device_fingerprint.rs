use crate::db::{Database, DeviceFingerprintRecord};
use crate::error::{AppError, AppResult};
use crate::services::device_fingerprint::DeviceFingerprintService;
use crate::services::event_bus::EventBus;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn list_device_fingerprints(
    db: State<'_, Database>,
) -> AppResult<Vec<DeviceFingerprintRecord>> {
    DeviceFingerprintService::list(&db)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

#[tauri::command]
pub async fn create_device_fingerprint(
    app: AppHandle,
    db: State<'_, Database>,
    name: String,
    seed: Option<DeviceFingerprintRecord>,
) -> AppResult<DeviceFingerprintRecord> {
    let fp = DeviceFingerprintService::create(&db, &name, seed)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    EventBus::emit_data_changed(
        &app,
        "device_fingerprints",
        "create",
        "device_fingerprint.create",
    );
    Ok(fp)
}

#[tauri::command]
pub async fn rename_device_fingerprint(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    name: String,
) -> AppResult<usize> {
    let rows = DeviceFingerprintService::rename(&db, &id, &name)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    if rows > 0 {
        EventBus::emit_data_changed(
            &app,
            "device_fingerprints",
            "rename",
            "device_fingerprint.rename",
        );
    }
    Ok(rows)
}

#[tauri::command]
pub async fn delete_device_fingerprint(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
) -> AppResult<usize> {
    let rows = DeviceFingerprintService::delete(&db, &id)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    if rows > 0 {
        EventBus::emit_data_changed(
            &app,
            "device_fingerprints",
            "delete",
            "device_fingerprint.delete",
        );
    }
    Ok(rows)
}

#[tauri::command]
pub async fn apply_device_fingerprint_to_account(
    app: AppHandle,
    db: State<'_, Database>,
    account_id: String,
    fingerprint_id: Option<String>,
) -> AppResult<usize> {
    let rows = DeviceFingerprintService::apply_to_account(&db, &account_id, fingerprint_id.as_deref())
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    if rows > 0 {
        EventBus::emit_data_changed(
            &app,
            "ide_accounts",
            "fingerprint_bound",
            "device_fingerprint.apply",
        );
    }
    Ok(rows)
}
