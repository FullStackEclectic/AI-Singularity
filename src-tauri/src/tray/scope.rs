use super::types::{TRAY_SCOPE_FILE, TrayScopeState};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn normalize_platform_scope(platforms: Vec<String>) -> Vec<String> {
    let mut list = platforms
        .into_iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    list.sort();
    list.dedup();
    list
}

fn tray_scope_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))?;
    fs::create_dir_all(&app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
    Ok(app_data_dir.join(TRAY_SCOPE_FILE))
}

fn load_tray_scope_state(app: &AppHandle) -> TrayScopeState {
    let path = match tray_scope_path(app) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!("[Tray] 读取托盘范围配置路径失败: {}", err);
            return TrayScopeState::default();
        }
    };
    if !path.exists() {
        return TrayScopeState::default();
    }
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[Tray] 读取托盘范围配置失败: {}", err);
            return TrayScopeState::default();
        }
    };
    if raw.trim().is_empty() {
        return TrayScopeState::default();
    }
    serde_json::from_str::<TrayScopeState>(&raw).unwrap_or_else(|err| {
        tracing::warn!("[Tray] 解析托盘范围配置失败: {}", err);
        TrayScopeState::default()
    })
}

fn save_tray_scope_state(app: &AppHandle, state: &TrayScopeState) -> Result<(), String> {
    let path = tray_scope_path(app)?;
    let raw =
        serde_json::to_string_pretty(state).map_err(|e| format!("序列化托盘范围失败: {}", e))?;
    fs::write(path, raw).map_err(|e| format!("写入托盘范围失败: {}", e))
}

pub(super) fn get_scope_platforms(app: &AppHandle) -> Vec<String> {
    normalize_platform_scope(load_tray_scope_state(app).platforms)
}

pub(super) fn set_scope_platforms(
    app: &AppHandle,
    platforms: Vec<String>,
) -> Result<Vec<String>, String> {
    let normalized = normalize_platform_scope(platforms);
    let state = TrayScopeState {
        platforms: normalized.clone(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    save_tray_scope_state(app, &state)?;
    Ok(normalized)
}
