use crate::services::skill::{SkillInfo, SkillService};

#[derive(serde::Serialize)]
pub struct SkillStorageInfo {
    pub primary_path: String,
    pub legacy_path: String,
    pub legacy_exists: bool,
}

#[tauri::command]
pub async fn list_skills() -> Result<Vec<SkillInfo>, String> {
    SkillService::list_skills().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_skill_storage_info() -> Result<SkillStorageInfo, String> {
    let primary = SkillService::get_primary_skills_dir();
    let legacy = SkillService::get_legacy_commands_dir();

    Ok(SkillStorageInfo {
        primary_path: primary.to_string_lossy().to_string(),
        legacy_path: legacy.to_string_lossy().to_string(),
        legacy_exists: legacy.exists(),
    })
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
