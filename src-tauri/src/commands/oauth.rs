use tauri::AppHandle;
use crate::services::oauth::OauthManager;

#[tauri::command]
pub fn start_oauth_flow(app: AppHandle, provider: String) -> Result<String, String> {
    OauthManager::start_oauth_flow(app, provider)
}
