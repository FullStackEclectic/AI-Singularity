use crate::error::AppResult;
use crate::models::{AiTool, ProviderConfig};
use crate::services::provider::ProviderService;
use tauri::State;
use crate::db::Database;

#[tauri::command]
pub fn get_providers(db: State<'_, Database>) -> AppResult<Vec<ProviderConfig>> {
    let service = ProviderService::new(&*db);
    service.list_providers()
}

#[tauri::command]
pub fn add_provider(provider: ProviderConfig, db: State<'_, Database>) -> AppResult<()> {
    let service = ProviderService::new(&*db);
    service.add_provider(provider)
}

#[tauri::command]
pub fn switch_provider(id: String, ai_tool: AiTool, db: State<'_, Database>) -> AppResult<()> {
    let service = ProviderService::new(&*db);
    service.switch_provider(&id, &ai_tool)
}

#[tauri::command]
pub fn delete_provider(id: String, db: State<'_, Database>) -> AppResult<()> {
    let service = ProviderService::new(&*db);
    service.delete_provider(&id)
}
