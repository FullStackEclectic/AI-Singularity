use crate::services::skill::{SkillService, SkillInfo};

#[tauri::command]
pub async fn list_skills() -> Result<Vec<SkillInfo>, String> {
    SkillService::list_skills().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_skill(url: String) -> Result<SkillInfo, String> {
    SkillService::install_skill(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_skill(id: String) -> Result<(), String> {
    SkillService::update_skill(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn uninstall_skill(id: String) -> Result<(), String> {
    SkillService::remove_skill(&id).map_err(|e| e.to_string())
}
