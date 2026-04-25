use super::{
    normalize_client_version_mode, WakeupHistoryItem, WakeupService, WakeupState, WakeupTask,
    WAKEUP_HISTORY_FILE, WAKEUP_STATE_FILE,
};
use crate::db::{Database, WakeupHistoryRow, WakeupTaskRow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const WAKEUP_GLOBAL_ENABLED_KEY: &str = "wakeup_global_enabled";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WakeupTaskConfig {
    #[serde(default)]
    reset_window: Option<String>,
    #[serde(default)]
    window_day_policy: Option<String>,
    #[serde(default)]
    window_fallback_policy: Option<String>,
    #[serde(default)]
    client_version_mode: Option<String>,
    #[serde(default)]
    client_version_fallback_mode: Option<String>,
    #[serde(default)]
    cron: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    retry_failed_times: Option<u8>,
    #[serde(default)]
    pause_after_failures: Option<u8>,
}

fn task_to_row(task: &WakeupTask) -> WakeupTaskRow {
    let config = WakeupTaskConfig {
        reset_window: Some(task.reset_window.clone()),
        window_day_policy: Some(task.window_day_policy.clone()),
        window_fallback_policy: Some(task.window_fallback_policy.clone()),
        client_version_mode: Some(task.client_version_mode.clone()),
        client_version_fallback_mode: Some(task.client_version_fallback_mode.clone()),
        cron: Some(task.cron.clone()),
        timeout_seconds: Some(task.timeout_seconds),
        retry_failed_times: Some(task.retry_failed_times),
        pause_after_failures: Some(task.pause_after_failures),
    };
    let config_json = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
    WakeupTaskRow {
        id: task.id.clone(),
        name: task.name.clone(),
        enabled: task.enabled,
        account_id: task.account_id.clone(),
        trigger_mode: task.trigger_mode.clone(),
        config_json,
        model: task.model.clone(),
        prompt: Some(task.prompt.clone()),
        command_template: task.command_template.clone(),
        notes: task.notes.clone(),
        last_run_at: task.last_run_at.clone(),
        last_status: task.last_status.clone(),
        last_category: task.last_category.clone(),
        last_message: task.last_message.clone(),
        consecutive_failures: task.consecutive_failures as i64,
        created_at: task.created_at.clone(),
        updated_at: task.updated_at.clone(),
    }
}

fn row_to_task(row: WakeupTaskRow) -> WakeupTask {
    let config: WakeupTaskConfig =
        serde_json::from_str(&row.config_json).unwrap_or_default();
    WakeupTask {
        id: row.id,
        name: row.name,
        enabled: row.enabled,
        account_id: row.account_id,
        trigger_mode: row.trigger_mode,
        reset_window: config.reset_window.unwrap_or_else(|| "primary_window".to_string()),
        window_day_policy: config.window_day_policy.unwrap_or_else(|| "all_days".to_string()),
        window_fallback_policy: config
            .window_fallback_policy
            .unwrap_or_else(|| "none".to_string()),
        client_version_mode: normalize_client_version_mode(
            &config.client_version_mode.unwrap_or_else(|| "auto".to_string()),
        ),
        client_version_fallback_mode: normalize_client_version_mode(
            &config
                .client_version_fallback_mode
                .unwrap_or_else(|| "auto".to_string()),
        ),
        command_template: row.command_template,
        model: row.model,
        prompt: row.prompt.unwrap_or_default(),
        cron: config.cron.unwrap_or_default(),
        notes: row.notes,
        timeout_seconds: config.timeout_seconds.unwrap_or(120),
        retry_failed_times: config.retry_failed_times.unwrap_or(0),
        pause_after_failures: config.pause_after_failures.unwrap_or(0),
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_run_at: row.last_run_at,
        last_status: row.last_status,
        last_category: row.last_category,
        last_message: row.last_message,
        consecutive_failures: row.consecutive_failures.max(0).min(u8::MAX as i64) as u8,
    }
}

fn history_item_to_row(item: &WakeupHistoryItem) -> WakeupHistoryRow {
    WakeupHistoryRow {
        id: item.id.clone(),
        run_id: item
            .run_id
            .clone()
            .unwrap_or_else(|| format!("legacy-{}", uuid::Uuid::new_v4())),
        task_id: item.task_id.clone(),
        task_name: item.task_name.clone(),
        account_id: item.account_id.clone(),
        model: item.model.clone(),
        status: item.status.clone(),
        category: item.category.clone(),
        message: item.message.clone(),
        attempts: 1,
        created_at: item.created_at.clone(),
    }
}

fn row_to_history_item(row: WakeupHistoryRow) -> WakeupHistoryItem {
    WakeupHistoryItem {
        id: row.id,
        run_id: Some(row.run_id),
        task_id: row.task_id,
        task_name: row.task_name,
        account_id: row.account_id,
        model: row.model,
        status: row.status,
        category: row.category,
        message: row.message,
        created_at: row.created_at,
    }
}

impl WakeupService {
    pub fn load_state(db: &Database) -> Result<WakeupState, String> {
        let enabled = db
            .get_account_setting(WAKEUP_GLOBAL_ENABLED_KEY)
            .map_err(|e| format!("读取 Wakeup 总开关失败: {}", e))?
            .map(|v| v == "true")
            .unwrap_or(false);
        let rows = db
            .list_wakeup_tasks()
            .map_err(|e| format!("读取 Wakeup 任务失败: {}", e))?;
        let tasks = rows.into_iter().map(row_to_task).collect();
        Ok(WakeupState { enabled, tasks })
    }

    pub fn save_state(db: &Database, mut state: WakeupState) -> Result<WakeupState, String> {
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
        let rows: Vec<WakeupTaskRow> = state.tasks.iter().map(task_to_row).collect();
        db.replace_wakeup_tasks(&rows)
            .map_err(|e| format!("写入 Wakeup 任务失败: {}", e))?;
        db.set_account_setting(
            WAKEUP_GLOBAL_ENABLED_KEY,
            if state.enabled { "true" } else { "false" },
        )
        .map_err(|e| format!("写入 Wakeup 总开关失败: {}", e))?;
        Ok(state)
    }

    pub fn upsert_task(db: &Database, task: &WakeupTask) -> Result<(), String> {
        let row = task_to_row(task);
        db.upsert_wakeup_task(&row)
            .map_err(|e| format!("更新 Wakeup 任务失败: {}", e))?;
        Ok(())
    }

    pub fn load_history(db: &Database) -> Result<Vec<WakeupHistoryItem>, String> {
        let rows = db
            .list_wakeup_history(None, super::MAX_HISTORY_ITEMS)
            .map_err(|e| format!("读取 Wakeup 历史失败: {}", e))?;
        Ok(rows.into_iter().map(row_to_history_item).collect())
    }

    pub fn add_history_items(
        db: &Database,
        items: Vec<WakeupHistoryItem>,
    ) -> Result<Vec<WakeupHistoryItem>, String> {
        if items.is_empty() {
            return Self::load_history(db);
        }
        let rows: Vec<WakeupHistoryRow> = items.iter().map(history_item_to_row).collect();
        db.append_wakeup_history(&rows)
            .map_err(|e| format!("写入 Wakeup 历史失败: {}", e))?;
        Self::load_history(db)
    }

    pub fn clear_history(db: &Database) -> Result<(), String> {
        db.clear_wakeup_history()
            .map_err(|e| format!("清空 Wakeup 历史失败: {}", e))?;
        Ok(())
    }

    /// Idempotent legacy JSON migration. Reads `wakeup_state.json` /
    /// `wakeup_history.json` if present, imports into the database, then
    /// renames them to `*.migrated.json` so subsequent boots skip the import.
    pub fn migrate_legacy_json(app_data_dir: &Path, db: &Database) -> Result<(), String> {
        migrate_state_json(app_data_dir, db)?;
        migrate_history_json(app_data_dir, db)?;
        Ok(())
    }
}

fn state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(WAKEUP_STATE_FILE)
}

fn history_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(WAKEUP_HISTORY_FILE)
}

fn migrate_state_json(app_data_dir: &Path, db: &Database) -> Result<(), String> {
    let path = state_path(app_data_dir);
    if !path.exists() {
        return Ok(());
    }
    let raw = match fs::read_to_string(&path) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[Wakeup] 旧 state JSON 读取失败，跳过迁移: {}", e);
            return Ok(());
        }
    };
    if raw.trim().is_empty() {
        let _ = fs::remove_file(&path);
        return Ok(());
    }
    let legacy: WakeupState = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[Wakeup] 旧 state JSON 解析失败，跳过迁移: {}", e);
            return Ok(());
        }
    };

    let existing = db.list_wakeup_tasks().unwrap_or_default();
    if existing.is_empty() {
        let rows: Vec<WakeupTaskRow> = legacy.tasks.iter().map(task_to_row).collect();
        if let Err(e) = db.replace_wakeup_tasks(&rows) {
            tracing::warn!("[Wakeup] 旧 state 写入 DB 失败: {}", e);
            return Ok(());
        }
        let _ = db.set_account_setting(
            WAKEUP_GLOBAL_ENABLED_KEY,
            if legacy.enabled { "true" } else { "false" },
        );
        tracing::info!("[Wakeup] 已迁移旧 state JSON，共 {} 条任务", legacy.tasks.len());
    }

    let archived = path.with_extension("migrated.json");
    if let Err(e) = fs::rename(&path, archived) {
        tracing::warn!("[Wakeup] 旧 state JSON 归档失败: {}", e);
    }
    Ok(())
}

fn migrate_history_json(app_data_dir: &Path, db: &Database) -> Result<(), String> {
    let path = history_path(app_data_dir);
    if !path.exists() {
        return Ok(());
    }
    let raw = match fs::read_to_string(&path) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[Wakeup] 旧 history JSON 读取失败，跳过迁移: {}", e);
            return Ok(());
        }
    };
    if raw.trim().is_empty() {
        let _ = fs::remove_file(&path);
        return Ok(());
    }
    let legacy: Vec<WakeupHistoryItem> = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[Wakeup] 旧 history JSON 解析失败，跳过迁移: {}", e);
            return Ok(());
        }
    };

    if !legacy.is_empty() {
        let rows: Vec<WakeupHistoryRow> = legacy.iter().map(history_item_to_row).collect();
        if let Err(e) = db.append_wakeup_history(&rows) {
            tracing::warn!("[Wakeup] 旧 history 写入 DB 失败: {}", e);
            return Ok(());
        }
        tracing::info!("[Wakeup] 已迁移旧 history JSON，共 {} 条", legacy.len());
    }

    let archived = path.with_extension("migrated.json");
    if let Err(e) = fs::rename(&path, archived) {
        tracing::warn!("[Wakeup] 旧 history JSON 归档失败: {}", e);
    }
    Ok(())
}
