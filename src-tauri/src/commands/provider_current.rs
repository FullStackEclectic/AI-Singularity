use crate::db::Database;
use crate::services::provider_current::ProviderCurrentService;
use tauri::State;

#[tauri::command]
pub async fn get_provider_current_account_id(
    db: State<'_, Database>,
    platform: String,
) -> Result<Option<String>, String> {
    ProviderCurrentService::get_current_account_id(&*db, &platform)
}
