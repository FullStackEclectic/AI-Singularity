use super::{
    normalize_client_version_mode, WakeupHistoryItem, WakeupService, WakeupState,
    MAX_HISTORY_ITEMS,
};
use chrono::Utc;
use std::fs;
use std::path::Path;

impl WakeupService {
    pub fn load_state(app_data_dir: &Path) -> Result<WakeupState, String> {
        let path = state_path(app_data_dir);
        if !path.exists() {
            return Ok(WakeupState::default());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("读取 Wakeup 状态失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(WakeupState::default());
        }
        serde_json::from_str(&raw).map_err(|e| format!("解析 Wakeup 状态失败: {}", e))
    }

    pub fn save_state(app_data_dir: &Path, mut state: WakeupState) -> Result<WakeupState, String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let now = Utc::now().to_rfc3339();
        for task in &mut state.tasks {
            if task.created_at.trim().is_empty() {
                task.created_at = now.clone();
            }
            task.client_version_mode = normalize_client_version_mode(&task.client_version_mode);
            task.client_version_fallback_mode =
                normalize_client_version_mode(&task.client_version_fallback_mode);
            task.updated_at = now.clone();
        }
        let content = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("序列化 Wakeup 状态失败: {}", e))?;
        fs::write(state_path(app_data_dir), content)
            .map_err(|e| format!("写入 Wakeup 状态失败: {}", e))?;
        Ok(state)
    }

    pub fn load_history(app_data_dir: &Path) -> Result<Vec<WakeupHistoryItem>, String> {
        let path = history_path(app_data_dir);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("读取 Wakeup 历史失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&raw).map_err(|e| format!("解析 Wakeup 历史失败: {}", e))
    }

    pub fn add_history_items(
        app_data_dir: &Path,
        items: Vec<WakeupHistoryItem>,
    ) -> Result<Vec<WakeupHistoryItem>, String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let mut current = Self::load_history(app_data_dir)?;
        current.extend(items);
        current.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        current.truncate(MAX_HISTORY_ITEMS);
        let content = serde_json::to_string_pretty(&current)
            .map_err(|e| format!("序列化 Wakeup 历史失败: {}", e))?;
        fs::write(history_path(app_data_dir), content)
            .map_err(|e| format!("写入 Wakeup 历史失败: {}", e))?;
        Ok(current)
    }

    pub fn clear_history(app_data_dir: &Path) -> Result<(), String> {
        let path = history_path(app_data_dir);
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("清空 Wakeup 历史失败: {}", e))?;
        }
        Ok(())
    }
}

pub(super) fn state_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join(super::WAKEUP_STATE_FILE)
}

pub(super) fn history_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join(super::WAKEUP_HISTORY_FILE)
}
