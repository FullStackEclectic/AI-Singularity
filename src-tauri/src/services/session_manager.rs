use rusqlite::types::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use sysinfo::System;

mod codex;
mod parsing;
mod sources;
mod trash;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZombieProcess {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub active_time_sec: u64,
    pub tool_type: String,
    pub cwd: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<u64>,
    #[serde(default)]
    pub full_content: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages_count: usize,
    pub filepath: String,
    pub tool_type: Option<String>,
    pub cwd: Option<String>,
    pub instance_id: Option<String>,
    pub instance_name: Option<String>,
    pub source_kind: Option<String>,
    pub has_tool_calls: bool,
    pub has_log_events: bool,
    #[serde(default)]
    pub latest_tool_name: Option<String>,
    #[serde(default)]
    pub latest_tool_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionTrashSummary {
    pub requested_count: usize,
    pub trashed_count: usize,
    pub trash_dir: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodexSessionRepairSummary {
    pub thread_count: usize,
    pub added_index_entries: usize,
    pub removed_stale_entries: usize,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodexThreadSyncItem {
    pub instance_id: String,
    pub instance_name: String,
    pub added_thread_count: usize,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodexThreadSyncSummary {
    pub instance_count: usize,
    pub thread_universe_count: usize,
    pub mutated_instance_count: usize,
    pub total_synced_thread_count: usize,
    pub items: Vec<CodexThreadSyncItem>,
    pub backup_dirs: Vec<String>,
    pub message: String,
}

pub struct SessionManager;

#[derive(Debug, Clone)]
struct CodexInstanceRef {
    id: String,
    name: String,
    data_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct ThreadRowData {
    columns: Vec<String>,
    values: Vec<Value>,
}

impl ThreadRowData {
    fn get_value(&self, column: &str) -> Option<&Value> {
        self.columns
            .iter()
            .position(|item| item == column)
            .and_then(|index| self.values.get(index))
    }

    fn get_text(&self, column: &str) -> Option<String> {
        match self.get_value(column)? {
            Value::Text(value) => Some(value.clone()),
            Value::Integer(value) => Some(value.to_string()),
            Value::Real(value) => Some(value.to_string()),
            _ => None,
        }
    }

    fn get_i64(&self, column: &str) -> Option<i64> {
        match self.get_value(column)? {
            Value::Integer(value) => Some(*value),
            Value::Text(value) => value.parse::<i64>().ok(),
            _ => None,
        }
    }

    fn set_text(&mut self, column: &str, value: String) {
        if let Some(index) = self.columns.iter().position(|item| item == column) {
            if let Some(slot) = self.values.get_mut(index) {
                *slot = Value::Text(value);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ThreadSnapshot {
    id: String,
    title: String,
    cwd: String,
    updated_at: Option<i64>,
    rollout_path: PathBuf,
    row_data: ThreadRowData,
    session_index_entry: JsonValue,
    source_root: PathBuf,
}

struct FormattedToolCall {
    preview: String,
    full_content: Option<String>,
    source_path: Option<String>,
    timestamp: Option<u64>,
}

impl SessionManager {
    /// 全域进程雷达：探测环境内存活的第三方 AI CLI 工具
    pub fn scan_zombie_processes() -> Vec<ZombieProcess> {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut zombies = Vec::new();

        for (pid, process) in sys.processes() {
            let cmd_args: Vec<String> = process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            let cmd = cmd_args.join(" ").to_lowercase();
            let cwd = process
                .cwd()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut tool_type = None;

            if cmd.contains("claude-cli") || (cmd.contains("claude") && cmd.contains("node")) {
                tool_type = Some("ClaudeCode");
            } else if cmd.contains("codex") {
                tool_type = Some("Codex");
            } else if cmd.contains("aider") && !cmd.contains("cargo") {
                tool_type = Some("Aider");
            } else if cmd.contains("gemini") {
                tool_type = Some("GeminiCLI");
            } else if cmd.contains("opencode") {
                tool_type = Some("OpenCode");
            }

            if let Some(tt) = tool_type {
                zombies.push(ZombieProcess {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().to_string(),
                    command: cmd_args.join(" "),
                    active_time_sec: process.run_time(),
                    tool_type: tt.to_string(),
                    cwd: cwd.clone(),
                });
            }
        }

        zombies.sort_by(|a, b| b.active_time_sec.cmp(&a.active_time_sec));
        zombies
    }

    /// 扫描 ~/.claude/projects/ 目录下的所有 jsonl
    pub fn list_sessions() -> Result<Vec<ChatSession>, String> {
        let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
        let mut sessions = Vec::new();
        Self::collect_claude_sessions(&home_dir, &mut sessions);
        Self::collect_codex_sessions(&home_dir, &mut sessions);
        Self::collect_gemini_tmp_sessions(&home_dir, &mut sessions);
        Self::collect_gemini_workspace_history(&home_dir, &mut sessions);

        let zombies = Self::scan_zombie_processes();
        for zombie in zombies {
            if zombie.tool_type == "Aider" && !zombie.cwd.is_empty() {
                let path = PathBuf::from(&zombie.cwd).join(".aider.chat.history.md");
                if path.exists() {
                    if let Ok(metadata) = fs::metadata(&path) {
                        let modified = metadata
                            .modified()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let content = fs::read_to_string(&path).unwrap_or_default();
                        let count = content.lines().count();

                        let project_name = PathBuf::from(&zombie.cwd)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();

                        if !sessions
                            .iter()
                            .any(|s| s.filepath == path.to_string_lossy().to_string())
                        {
                            sessions.push(ChatSession {
                                id: format!("aider-{}", zombie.pid),
                                title: format!("Aider // {}", project_name),
                                created_at: modified,
                                updated_at: modified,
                                messages_count: count,
                                filepath: path.to_string_lossy().into_owned(),
                                tool_type: Some("Aider".to_string()),
                                cwd: Some(zombie.cwd.clone()),
                                instance_id: None,
                                instance_name: None,
                                source_kind: Some("transcript".to_string()),
                                has_tool_calls: false,
                                has_log_events: false,
                                latest_tool_name: None,
                                latest_tool_status: None,
                            });
                        }
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

}
