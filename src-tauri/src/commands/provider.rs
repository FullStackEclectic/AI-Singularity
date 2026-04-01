use crate::error::AppResult;
use crate::models::ProviderConfig;
use crate::services::provider::ProviderService;
use tauri::State;
use crate::db::Database;

#[tauri::command]
pub fn get_providers(db: State<'_, Database>) -> AppResult<Vec<ProviderConfig>> {
    ProviderService::new(&*db).list_providers()
}

#[tauri::command]
pub fn add_provider(provider: ProviderConfig, db: State<'_, Database>) -> AppResult<()> {
    ProviderService::new(&*db).add_provider(provider)
}

#[tauri::command]
pub fn update_provider(provider: ProviderConfig, db: State<'_, Database>) -> AppResult<()> {
    ProviderService::new(&*db).update_provider(provider)
}

/// 切换激活 Provider（不再需要 ai_tool，后端基于 id 全局互斥）
#[tauri::command]
pub fn switch_provider(id: String, db: State<'_, Database>) -> AppResult<()> {
    ProviderService::new(&*db).switch_provider(&id)
}

#[tauri::command]
pub fn delete_provider(id: String, db: State<'_, Database>) -> AppResult<()> {
    ProviderService::new(&*db).delete_provider(&id)
}
