use crate::services::env_checker::{EnvChecker, EnvConflict};
use crate::AppError;

#[tauri::command]
pub async fn check_system_env_conflicts(app_name: String) -> Result<Vec<EnvConflict>, AppError> {
    let conflicts = EnvChecker::check_env_conflicts(&app_name);
    Ok(conflicts)
}
