use crate::db::Database;
use crate::error::AppResult;
use crate::services::backup::{BackupData, BackupService};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn export_config(app: AppHandle, db: State<'_, Database>) -> AppResult<BackupData> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    BackupService::new(&*db, app_data_dir).export_config()
}

#[tauri::command]
pub fn import_config(json_data: String, app: AppHandle, db: State<'_, Database>) -> AppResult<()> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    BackupService::new(&*db, app_data_dir).import_config(&json_data)?;
    crate::tray::update_tray_menu(&app);
    Ok(())
}
