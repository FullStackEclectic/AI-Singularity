use crate::db::Database;
use crate::error::AppResult;
use crate::services::extension_import::{
    ExtensionImportService, ExtensionImportStats, ExtensionScanResult,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn scan_extension_credentials() -> AppResult<Vec<ExtensionScanResult>> {
    Ok(ExtensionImportService::scan())
}

#[tauri::command]
pub async fn import_from_extension(
    app: AppHandle,
    db: State<'_, Database>,
) -> AppResult<ExtensionImportStats> {
    Ok(ExtensionImportService::import_all(&db, Some(&app)))
}
