use crate::error::AppResult;
use crate::models::AlertItem;
use crate::services::alert::AlertService;
use crate::db::Database;
use tauri::State;

#[tauri::command]
pub fn get_alerts(db: State<'_, Database>) -> AppResult<Vec<AlertItem>> {
    let service = AlertService::new(&db);
    Ok(service.get_alerts())
}
