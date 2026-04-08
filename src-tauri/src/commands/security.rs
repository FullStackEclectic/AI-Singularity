use tauri::State;
use crate::error::AppResult;
use crate::db::Database;
use crate::models::{IpAccessLog, IpRule};
use crate::services::security_db::SecurityDbService;
use std::sync::Arc;

#[tauri::command]
pub async fn get_ip_access_logs(db: State<'_, Database>, limit: Option<i64>) -> AppResult<Vec<IpAccessLog>> {
    let service = SecurityDbService::new(&Arc::new((*db).clone()));
    service.get_access_logs(limit.unwrap_or(200))
}

#[tauri::command]
pub async fn clear_ip_access_logs(db: State<'_, Database>) -> AppResult<()> {
    let service = SecurityDbService::new(&Arc::new((*db).clone()));
    service.clear_access_logs()
}

#[tauri::command]
pub async fn get_ip_rules(db: State<'_, Database>) -> AppResult<Vec<IpRule>> {
    let service = SecurityDbService::new(&Arc::new((*db).clone()));
    service.get_all_rules()
}

#[tauri::command]
pub async fn add_ip_rule(
    db: State<'_, Database>,
    ip_cidr: String,
    rule_type: String,
    notes: Option<String>,
) -> AppResult<()> {
    let service = SecurityDbService::new(&Arc::new((*db).clone()));
    service.add_rule(&ip_cidr, &rule_type, notes.as_deref())
}

#[tauri::command]
pub async fn delete_ip_rule(db: State<'_, Database>, id: String) -> AppResult<()> {
    let service = SecurityDbService::new(&Arc::new((*db).clone()));
    service.delete_rule(&id)
}

#[tauri::command]
pub async fn toggle_ip_rule(db: State<'_, Database>, id: String, active: bool) -> AppResult<()> {
    let service = SecurityDbService::new(&Arc::new((*db).clone()));
    service.toggle_rule(&id, active)
}
