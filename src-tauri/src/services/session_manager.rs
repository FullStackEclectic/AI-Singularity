use crate::services::codex_instance_store::CodexInstanceStore;
use chrono::{DateTime, Utc};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sysinfo::System;

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

        // 去重逻辑，有时候进程会 spawning 树，这里简单过滤
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

        // 动态侦测进程并吸血：通过僵尸雷达拉取 Aider 缓存
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

                        // 防止重复推入同一个项目的 Aider 会话（多个相同 cwd 的 aider 进程）
                        if !sessions
                            .iter()
                            .any(|s| s.filepath == path.to_string_lossy().to_string())
                        {
                            sessions.push(ChatSession {
                                id: format!("aider-{}", zombie.pid),
                                title: format!("Aider // {}", project_name),
                                created_at: modified,
                                updated_at: modified,
                                messages_count: count, // markdown lines count
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

        // 近期活跃的排前
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    fn collect_gemini_tmp_sessions(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
        let tmp_dir = home_dir.join(".gemini").join("tmp");
        if !tmp_dir.exists() {
            return;
        }

        let Ok(entries) = fs::read_dir(&tmp_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let marker = path.join(".project_root");
            let chats_dir = path.join("chats");
            if !marker.exists() || !chats_dir.exists() {
                continue;
            }

            let workspace_root = fs::read_to_string(&marker)
                .unwrap_or_default()
                .trim()
                .to_string();
            if workspace_root.is_empty() {
                continue;
            }

            let Ok(chat_entries) = fs::read_dir(&chats_dir) else {
                continue;
            };

            for chat_entry in chat_entries.flatten() {
                let chat_path = chat_entry.path();
                if !chat_path.is_file()
                    || chat_path.extension().and_then(|ext| ext.to_str()) != Some("json")
                {
                    continue;
                }

                let Ok(raw) = fs::read_to_string(&chat_path) else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
                    continue;
                };
                let Some(messages) = value.get("messages").and_then(|item| item.as_array()) else {
                    continue;
                };

                let first_user_text = messages.iter().find_map(Self::extract_gemini_message_text);
                let title_seed = first_user_text
                    .filter(|text| !text.trim().is_empty())
                    .map(|text| truncate_single_line(&text, 42))
                    .unwrap_or_else(|| {
                        PathBuf::from(&workspace_root)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| {
                                chat_path
                                    .file_stem()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "Gemini 会话".to_string())
                            })
                    });

                let created_at = value
                    .get("startTime")
                    .and_then(|item| item.as_str())
                    .and_then(parse_rfc3339_seconds)
                    .unwrap_or_else(|| file_timestamp_seconds(&chat_path, false));
                let updated_at = value
                    .get("lastUpdated")
                    .and_then(|item| item.as_str())
                    .and_then(parse_rfc3339_seconds)
                    .unwrap_or_else(|| file_timestamp_seconds(&chat_path, true));

                let message_count = messages
                    .iter()
                    .filter(|item| item.get("type").and_then(|value| value.as_str()).is_some())
                    .count();
                let latest_tool_call = messages
                    .iter()
                    .flat_map(|item| item.get("toolCalls").and_then(|value| value.as_array()).into_iter().flatten())
                    .filter_map(|tool| {
                        let name = tool.get("name").and_then(|value| value.as_str())?;
                        let status = tool
                            .get("status")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown");
                        Some((name.to_string(), status.to_string()))
                    })
                    .last();

                sessions.push(ChatSession {
                    id: value
                        .get("sessionId")
                        .and_then(|item| item.as_str())
                        .map(|item| format!("gemini-session-{}", item))
                        .unwrap_or_else(|| {
                            format!(
                                "gemini-session-{}",
                                chat_path
                                    .file_stem()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "unknown".to_string())
                            )
                        }),
                    title: format!("Gemini // {}", title_seed),
                    created_at,
                    updated_at,
                    messages_count: message_count,
                    filepath: chat_path.to_string_lossy().into_owned(),
                    tool_type: Some("GeminiCLI".to_string()),
                    cwd: Some(workspace_root.clone()),
                    instance_id: None,
                    instance_name: Some("聊天转录".to_string()),
                    source_kind: Some("transcript".to_string()),
                    has_tool_calls: messages.iter().any(|item| {
                        item.get("toolCalls")
                            .and_then(|value| value.as_array())
                            .is_some_and(|items| !items.is_empty())
                    }),
                    has_log_events: path.join("logs.json").exists(),
                    latest_tool_name: latest_tool_call.as_ref().map(|item| item.0.clone()),
                    latest_tool_status: latest_tool_call.as_ref().map(|item| item.1.clone()),
                });
            }
        }
    }

    fn collect_gemini_workspace_history(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
        let history_dir = home_dir.join(".gemini").join("history");
        if !history_dir.exists() {
            return;
        }

        let Ok(entries) = fs::read_dir(&history_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let marker = path.join(".project_root");
            if !marker.exists() {
                continue;
            }

            let Ok(workspace_root) = fs::read_to_string(&marker) else {
                continue;
            };
            let workspace_root = workspace_root.trim().to_string();
            if workspace_root.is_empty() {
                continue;
            }

            let metadata = match fs::metadata(&marker) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            let modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let created = metadata
                .created()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let title = PathBuf::from(&workspace_root)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry.file_name().to_string_lossy().into_owned());

            sessions.push(ChatSession {
                id: format!("gemini-history-{}", entry.file_name().to_string_lossy()),
                title: format!("Gemini // {}", title),
                created_at: created,
                updated_at: modified,
                messages_count: 0,
                filepath: marker.to_string_lossy().into_owned(),
                tool_type: Some("GeminiCLI".to_string()),
                cwd: Some(workspace_root),
                instance_id: None,
                instance_name: Some("工作区历史".to_string()),
                source_kind: Some("workspace_history".to_string()),
                has_tool_calls: false,
                has_log_events: false,
                latest_tool_name: None,
                latest_tool_status: None,
            });
        }
    }

    pub fn move_sessions_to_trash(filepaths: Vec<String>) -> Result<SessionTrashSummary, String> {
        let requested: Vec<String> = filepaths
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        if requested.is_empty() {
            return Err("请至少选择一条会话".to_string());
        }

        let sessions = Self::list_sessions()?;
        let selected = sessions
            .into_iter()
            .filter(|session| requested.iter().any(|path| path == &session.filepath))
            .collect::<Vec<_>>();

        if selected.is_empty() {
            return Ok(SessionTrashSummary {
                requested_count: requested.len(),
                trashed_count: 0,
                trash_dir: String::new(),
                message: "所选会话不存在，无需处理".to_string(),
            });
        }

        let trash_root = Self::create_trash_root_dir()?;
        let mut trashed_count = 0usize;
        let mut codex_ids = Vec::new();

        for session in &selected {
            let source = PathBuf::from(&session.filepath);
            if !source.exists() {
                continue;
            }

            let tool = session
                .tool_type
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());
            let safe_tool = Self::sanitize_for_file_name(&tool);
            let safe_id = Self::sanitize_for_file_name(&session.id);
            let entry_dir = trash_root.join(format!("{}--{}", safe_tool, safe_id));
            fs::create_dir_all(&entry_dir)
                .map_err(|e| format!("创建废纸篓目录失败 ({}): {}", entry_dir.display(), e))?;

            let filename = source
                .file_name()
                .map(|name| name.to_os_string())
                .unwrap_or_else(|| std::ffi::OsString::from("session.dat"));
            let target = entry_dir.join(filename);

            fs::rename(&source, &target).map_err(|e| {
                format!(
                    "移动会话到废纸篓失败 ({} -> {}): {}",
                    source.display(),
                    target.display(),
                    e
                )
            })?;

            let manifest = serde_json::json!({
                "id": session.id,
                "title": session.title,
                "tool_type": session.tool_type,
                "cwd": session.cwd,
                "original_path": session.filepath,
                "trashed_at": Utc::now().to_rfc3339(),
            });

            fs::write(
                entry_dir.join("manifest.json"),
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&manifest)
                        .map_err(|e| format!("序列化废纸篓清单失败: {}", e))?
                ),
            )
            .map_err(|e| format!("写入废纸篓清单失败: {}", e))?;

            if session.tool_type.as_deref() == Some("Codex") {
                codex_ids.push(session.id.clone());
            }
            trashed_count += 1;
        }

        let mut codex_ids_by_instance = HashMap::<String, Vec<String>>::new();
        for session in &selected {
            if session.tool_type.as_deref() == Some("Codex") {
                if let Some(instance_id) = &session.instance_id {
                    codex_ids_by_instance
                        .entry(instance_id.clone())
                        .or_default()
                        .push(session.id.clone());
                }
            }
        }

        if !codex_ids_by_instance.is_empty() {
            let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
            let instances = Self::collect_codex_instances(&home_dir)?;
            for (instance_id, ids) in codex_ids_by_instance {
                if let Some(instance) = instances.iter().find(|item| item.id == instance_id) {
                    Self::remove_codex_threads_for_instance(instance, &ids)?;
                    Self::rewrite_codex_session_index_without_ids_for_instance(instance, &ids)?;
                }
            }
        }

        Ok(SessionTrashSummary {
            requested_count: requested.len(),
            trashed_count,
            trash_dir: trash_root.to_string_lossy().into_owned(),
            message: format!("已将 {} 条会话移到废纸篓", trashed_count),
        })
    }

    pub fn repair_codex_session_index() -> Result<CodexSessionRepairSummary, String> {
        let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
        let instances = Self::collect_codex_instances(&home_dir)?;

        let mut total_threads = 0usize;
        let mut total_added = 0usize;
        let mut total_removed = 0usize;

        for instance in &instances {
            let (threads, added, removed) =
                Self::repair_codex_session_index_for_instance(instance)?;
            total_threads += threads;
            total_added += added;
            total_removed += removed;
        }

        Ok(CodexSessionRepairSummary {
            thread_count: total_threads,
            added_index_entries: total_added,
            removed_stale_entries: total_removed,
            message: if total_added == 0 && total_removed == 0 {
                "所有 Codex 实例的会话索引已经是最新状态".to_string()
            } else {
                format!(
                    "Codex 会话索引已修复：新增 {} 条，清理 {} 条失效索引",
                    total_added, total_removed
                )
            },
        })
    }

    pub fn sync_codex_threads_across_instances() -> Result<CodexThreadSyncSummary, String> {
        let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
        let instances = Self::collect_codex_instances(&home_dir)?;
        if instances.len() < 2 {
            return Err("至少需要两个 Codex 实例才能同步线程".to_string());
        }

        let mut thread_universe = HashMap::<String, ThreadSnapshot>::new();
        let mut existing_ids_by_instance = HashMap::<String, HashSet<String>>::new();

        for instance in &instances {
            let snapshots = Self::load_codex_thread_snapshots(instance)?;
            let ids = snapshots
                .iter()
                .map(|item| item.id.clone())
                .collect::<HashSet<_>>();
            for snapshot in snapshots {
                thread_universe
                    .entry(snapshot.id.clone())
                    .or_insert(snapshot);
            }
            existing_ids_by_instance.insert(instance.id.clone(), ids);
        }

        let mut universe_ids = thread_universe.keys().cloned().collect::<Vec<_>>();
        universe_ids.sort();

        let mut items = Vec::with_capacity(instances.len());
        let mut backup_dirs = Vec::new();
        let mut mutated_instance_count = 0usize;
        let mut total_synced_thread_count = 0usize;

        for instance in &instances {
            let existing_ids = existing_ids_by_instance
                .get(&instance.id)
                .cloned()
                .unwrap_or_default();
            let missing_snapshots = universe_ids
                .iter()
                .filter(|id| !existing_ids.contains(*id))
                .filter_map(|id| thread_universe.get(id).cloned())
                .collect::<Vec<_>>();

            if missing_snapshots.is_empty() {
                items.push(CodexThreadSyncItem {
                    instance_id: instance.id.clone(),
                    instance_name: instance.name.clone(),
                    added_thread_count: 0,
                    backup_dir: None,
                });
                continue;
            }

            let backup_dir =
                Self::sync_missing_codex_threads_to_instance(instance, &missing_snapshots)?;
            let backup_dir_string = backup_dir.to_string_lossy().to_string();
            backup_dirs.push(backup_dir_string.clone());
            mutated_instance_count += 1;
            total_synced_thread_count += missing_snapshots.len();

            items.push(CodexThreadSyncItem {
                instance_id: instance.id.clone(),
                instance_name: instance.name.clone(),
                added_thread_count: missing_snapshots.len(),
                backup_dir: Some(backup_dir_string),
            });
        }

        Ok(CodexThreadSyncSummary {
            instance_count: instances.len(),
            thread_universe_count: thread_universe.len(),
            mutated_instance_count,
            total_synced_thread_count,
            items,
            backup_dirs,
            message: if total_synced_thread_count == 0 {
                "所有 Codex 实例已是最新，无需同步线程".to_string()
            } else {
                format!(
                    "已为 {} 个实例补齐 {} 条线程",
                    mutated_instance_count, total_synced_thread_count
                )
            },
        })
    }

    fn collect_claude_sessions(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
        let projects_dir = home_dir.join(".claude").join("projects");
        if !projects_dir.exists() {
            return;
        }

        if let Ok(entries) = fs::read_dir(projects_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let proj_path = entry.path();
                    if let Ok(files) = fs::read_dir(proj_path) {
                        for file_entry in files.flatten() {
                            let path = file_entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                                if let Some(session) = Self::parse_claude_session(&path) {
                                    sessions.push(session);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn parse_claude_session(path: &PathBuf) -> Option<ChatSession> {
        let metadata = fs::metadata(path).ok()?;
        let modified = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let created = metadata
            .created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let content = fs::read_to_string(path).ok()?;
        let mut cwd: Option<String> = None;
        let mut title: Option<String> = None;
        let mut message_count = 0usize;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };

            if cwd.is_none() {
                cwd = val
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| val.get("sessionId").and_then(|_| None));
            }

            if val.get("type").and_then(|v| v.as_str()) == Some("user") {
                message_count += 1;

                let is_meta = val.get("isMeta").and_then(|v| v.as_bool()).unwrap_or(false);
                if !is_meta && title.is_none() {
                    title = Self::extract_text_from_content(val.pointer("/message/content"))
                        .or_else(|| {
                            val.pointer("/message/content")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .and_then(Self::sanitize_session_title);
                }
            } else if val.get("type").and_then(|v| v.as_str()) == Some("assistant") {
                message_count += 1;
            }
        }

        let fallback_title = cwd
            .as_ref()
            .and_then(|c| {
                PathBuf::from(c)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .or_else(|| {
                path.parent()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            })
            .unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });

        Some(ChatSession {
            id: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            title: format!("Claude // {}", title.unwrap_or(fallback_title)),
            created_at: created,
            updated_at: modified,
            messages_count: message_count,
            filepath: path.to_string_lossy().into_owned(),
            tool_type: Some("ClaudeCode".to_string()),
            cwd,
            instance_id: None,
            instance_name: None,
            source_kind: Some("transcript".to_string()),
            has_tool_calls: false,
            has_log_events: false,
            latest_tool_name: None,
            latest_tool_status: None,
        })
    }

    fn collect_codex_sessions(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
        let Ok(instances) = Self::collect_codex_instances(home_dir) else {
            return;
        };

        for instance in instances {
            if let Ok(instance_sessions) = Self::load_codex_sessions_for_instance(&instance) {
                sessions.extend(instance_sessions);
            }
        }
    }

    fn collect_codex_instances(home_dir: &PathBuf) -> Result<Vec<CodexInstanceRef>, String> {
        let mut instances = vec![CodexInstanceRef {
            id: "__default__".to_string(),
            name: "默认实例".to_string(),
            data_dir: home_dir.join(".codex"),
        }];

        for item in CodexInstanceStore::list_instances()? {
            instances.push(CodexInstanceRef {
                id: item.id,
                name: item.name,
                data_dir: PathBuf::from(item.user_data_dir),
            });
        }

        Ok(instances)
    }

    fn load_codex_sessions_for_instance(
        instance: &CodexInstanceRef,
    ) -> Result<Vec<ChatSession>, String> {
        let snapshots = Self::load_codex_thread_snapshots(instance)?;
        Ok(snapshots
            .into_iter()
            .map(|snapshot| ChatSession {
                id: snapshot.id,
                title: format!("Codex // {}", snapshot.title),
                created_at: snapshot
                    .row_data
                    .get_i64("created_at")
                    .unwrap_or_default()
                    .max(0) as u64,
                updated_at: snapshot.updated_at.unwrap_or_default().max(0) as u64,
                messages_count: fs::read_to_string(&snapshot.rollout_path)
                    .map(|content| content.lines().count())
                    .unwrap_or(0),
                filepath: snapshot.rollout_path.to_string_lossy().into_owned(),
                tool_type: Some("Codex".to_string()),
                cwd: Some(snapshot.cwd),
                instance_id: Some(instance.id.clone()),
                instance_name: Some(instance.name.clone()),
                source_kind: Some("transcript".to_string()),
                has_tool_calls: false,
                has_log_events: false,
                latest_tool_name: None,
                latest_tool_status: None,
            })
            .collect())
    }

    fn normalize_codex_cwd(cwd: &str) -> String {
        cwd.strip_prefix(r"\\?\").unwrap_or(cwd).to_string()
    }

    fn load_codex_thread_snapshots(
        instance: &CodexInstanceRef,
    ) -> Result<Vec<ThreadSnapshot>, String> {
        let db_path = instance.data_dir.join("state_5.sqlite");
        if !db_path.exists() {
            return Ok(vec![]);
        }

        let conn = Self::open_readonly_connection(&db_path)?;
        let columns = Self::read_thread_columns(&conn)?;
        let select_columns = columns
            .iter()
            .map(|column| Self::quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!("SELECT {} FROM threads WHERE archived = 0", select_columns);
        let mut statement = conn
            .prepare(&query)
            .map_err(|e| format!("读取 Codex 线程失败 ({}): {}", instance.name, e))?;
        let mut rows = statement
            .query([])
            .map_err(|e| format!("查询 Codex 线程失败 ({}): {}", instance.name, e))?;
        let session_index_map =
            Self::read_codex_session_index_map(&instance.data_dir.join("session_index.jsonl"));

        let mut snapshots = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("迭代 Codex 线程失败 ({}): {}", instance.name, e))?
        {
            let mut values = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                values.push(
                    row.get::<usize, Value>(index).map_err(|e| {
                        format!("解析 Codex 线程记录失败 ({}): {}", instance.name, e)
                    })?,
                );
            }

            let row_data = ThreadRowData {
                columns: columns.clone(),
                values,
            };
            let id = row_data
                .get_text("id")
                .ok_or_else(|| format!("Codex 线程缺少 id 字段 ({})", instance.name))?;
            let rollout_path = row_data.get_text("rollout_path").ok_or_else(|| {
                format!("Codex 线程 {} 缺少 rollout_path ({})", id, instance.name)
            })?;
            let cwd = row_data
                .get_text("cwd")
                .map(|value| Self::normalize_codex_cwd(&value))
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "未知工作区".to_string());
            let title =
                Self::sanitize_session_title(row_data.get_text("title").unwrap_or_default())
                    .or_else(|| {
                        Self::sanitize_session_title(
                            row_data.get_text("first_user_message").unwrap_or_default(),
                        )
                    })
                    .or_else(|| {
                        session_index_map
                            .get(&id)
                            .and_then(|value| value.get("thread_name"))
                            .and_then(|value| value.as_str())
                            .map(|value| value.to_string())
                            .and_then(Self::sanitize_session_title)
                    })
                    .or_else(|| {
                        PathBuf::from(&cwd)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| id.clone());
            let updated_at = row_data.get_i64("updated_at");
            let session_index_entry = session_index_map.get(&id).cloned().unwrap_or_else(|| {
                serde_json::json!({
                    "id": id,
                    "thread_name": title,
                    "updated_at": Self::format_index_timestamp(updated_at.unwrap_or_default()),
                })
            });

            snapshots.push(ThreadSnapshot {
                id,
                title,
                cwd,
                updated_at,
                rollout_path: PathBuf::from(rollout_path),
                row_data,
                session_index_entry,
                source_root: instance.data_dir.clone(),
            });
        }

        Ok(snapshots)
    }

    fn open_readonly_connection(db_path: &PathBuf) -> Result<Connection, String> {
        Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("打开只读数据库失败 ({}): {}", db_path.display(), e))
    }

    fn read_thread_columns(connection: &Connection) -> Result<Vec<String>, String> {
        let mut statement = connection
            .prepare("PRAGMA table_info(threads)")
            .map_err(|e| format!("读取 threads 表结构失败: {}", e))?;
        let mut rows = statement
            .query([])
            .map_err(|e| format!("查询 threads 表结构失败: {}", e))?;
        let mut columns = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("解析 threads 表结构失败: {}", e))?
        {
            columns.push(
                row.get::<usize, String>(1)
                    .map_err(|e| format!("解析 threads 列失败: {}", e))?,
            );
        }

        if columns.is_empty() {
            return Err("threads 表不存在或没有列定义".to_string());
        }

        Ok(columns)
    }

    fn quote_identifier(value: &str) -> String {
        format!("\"{}\"", value.replace('"', "\"\""))
    }

    fn read_codex_session_index_map(path: &PathBuf) -> HashMap<String, JsonValue> {
        let mut entries = HashMap::new();
        let Ok(content) = fs::read_to_string(path) else {
            return entries;
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<JsonValue>(trimmed) else {
                continue;
            };
            let Some(id) = value.get("id").and_then(|item| item.as_str()) else {
                continue;
            };
            entries.insert(id.to_string(), value);
        }

        entries
    }

    fn remove_codex_threads_for_instance(
        instance: &CodexInstanceRef,
        session_ids: &[String],
    ) -> Result<(), String> {
        let db_path = instance.data_dir.join("state_5.sqlite");
        if !db_path.exists() {
            return Ok(());
        }

        let mut connection = Connection::open(&db_path)
            .map_err(|e| format!("打开 Codex 数据库失败 ({}): {}", db_path.display(), e))?;
        let tx = connection
            .transaction()
            .map_err(|e| format!("开启 Codex 会话删除事务失败: {}", e))?;

        for session_id in session_ids {
            tx.execute("DELETE FROM threads WHERE id = ?1", [session_id.as_str()])
                .map_err(|e| format!("删除 Codex 会话记录失败 ({}): {}", session_id, e))?;
        }

        tx.commit()
            .map_err(|e| format!("提交 Codex 会话删除事务失败: {}", e))?;
        Ok(())
    }

    fn rewrite_codex_session_index_without_ids_for_instance(
        instance: &CodexInstanceRef,
        session_ids: &[String],
    ) -> Result<(), String> {
        let path = instance.data_dir.join("session_index.jsonl");
        if !path.exists() {
            return Ok(());
        }

        let removed_ids = session_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let content = fs::read_to_string(&path).map_err(|e| {
            format!(
                "读取 Codex session_index.jsonl 失败 ({}): {}",
                path.display(),
                e
            )
        })?;

        let retained = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return false;
                }
                match serde_json::from_str::<JsonValue>(trimmed) {
                    Ok(value) => value
                        .get("id")
                        .and_then(JsonValue::as_str)
                        .map(|id| !removed_ids.contains(id))
                        .unwrap_or(true),
                    Err(_) => true,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let final_content = if retained.is_empty() {
            String::new()
        } else {
            format!("{}\n", retained)
        };

        fs::write(&path, final_content).map_err(|e| {
            format!(
                "重写 Codex session_index.jsonl 失败 ({}): {}",
                path.display(),
                e
            )
        })?;
        Ok(())
    }

    fn create_trash_root_dir() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
        let root = home
            .join(".ai-singularity")
            .join("session-trash")
            .join(Utc::now().format("%Y%m%d-%H%M%S").to_string());
        fs::create_dir_all(&root)
            .map_err(|e| format!("创建会话废纸篓目录失败 ({}): {}", root.display(), e))?;
        Ok(root)
    }

    fn sanitize_for_file_name(input: &str) -> String {
        input
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                    ch
                } else {
                    '-'
                }
            })
            .collect()
    }

    /// 获取单个 jsonl 文件的聊天对话
    pub fn get_session_details(filepath: &str) -> Result<Vec<ChatMessage>, String> {
        let path = PathBuf::from(filepath);
        if !path.exists() {
            return Err("File not found".into());
        }

        if path.file_name().and_then(|name| name.to_str()) == Some(".project_root") {
            let workspace_root = fs::read_to_string(&path)
                .unwrap_or_default()
                .trim()
                .to_string();
            let mut messages = vec![ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "这是 Gemini CLI 的工作区历史索引，而不是聊天消息转录。\n\n工作区路径：{}\n\n当前已确认 `~/.gemini/history/*/.project_root` 会记录历史工作区，完整聊天转录会优先从 `~/.gemini/tmp/*/chats/session-*.json` 读取。",
                    if workspace_root.is_empty() { "未知" } else { &workspace_root }
                ),
                timestamp: None,
                full_content: None,
                source_path: None,
            }];

            let history_dir = path.parent().unwrap_or(&path);
            let sibling_files = fs::read_dir(history_dir)
                .ok()
                .into_iter()
                .flat_map(|entries| entries.flatten())
                .filter_map(|entry| {
                    let child = entry.path();
                    if child.is_file()
                        && child.file_name().and_then(|name| name.to_str()) != Some(".project_root")
                    {
                        child.file_name().map(|name| name.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if !sibling_files.is_empty() {
                messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: format!(
                        "同目录还发现了这些文件，可继续排查 Gemini 历史目录结构：\n\n{}",
                        sibling_files
                            .iter()
                            .take(12)
                            .map(|item| format!("- {}", item))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                    timestamp: None,
                    full_content: None,
                    source_path: None,
                });
            }

            let preview_candidates = fs::read_dir(history_dir)
                .ok()
                .into_iter()
                .flat_map(|entries| entries.flatten())
                .map(|entry| entry.path())
                .filter(|candidate| candidate.is_file())
                .filter(|candidate| {
                    candidate.file_name().and_then(|name| name.to_str()) != Some(".project_root")
                })
                .filter(|candidate| is_gemini_history_preview_candidate(candidate))
                .take(3)
                .collect::<Vec<_>>();

            for candidate in preview_candidates {
                if let Ok(raw) = fs::read_to_string(&candidate) {
                    let preview = raw
                        .lines()
                        .take(40)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .chars()
                        .take(1500)
                        .collect::<String>();
                    if !preview.trim().is_empty() {
                        messages.push(ChatMessage {
                            role: "system".to_string(),
                            content: format!(
                                "历史目录文件预览：{}\n\n{}",
                                candidate
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| candidate.to_string_lossy().into_owned()),
                                preview
                            ),
                            timestamp: None,
                            full_content: None,
                            source_path: Some(candidate.to_string_lossy().into_owned()),
                        });
                    }
                }
            }

            return Ok(messages);
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut messages = Vec::new();

        if filepath.ends_with(".json") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if value.get("sessionId").is_some() && value.get("messages").and_then(|item| item.as_array()).is_some() {
                    return Ok(Self::parse_gemini_session_messages(&value, &path));
                }
            }
        }

        // 兼容 Aider 离线 Markdown 格式
        if filepath.ends_with(".md") {
            // Aider 的聊天记录通常是很长的 Markdown，如果没有明显结构，我们可以把它作为一个超级 Block 返回
            // 或者用正则切割 USER 和 ASSISTANT 块。由于 Aider 常以 `> USER:` 和 `> ASSISTANT:` 作为交互分隔。
            let mut current_role = "system".to_string();
            let mut current_text = String::new();

            for line in content.lines() {
                if line.starts_with("> USER:") {
                    if !current_text.trim().is_empty() {
                        messages.push(ChatMessage {
                            role: current_role,
                            content: current_text.clone(),
                            timestamp: None,
                            full_content: None,
                            source_path: None,
                        });
                    }
                    current_role = "user".to_string();
                    current_text = String::new();
                } else if line.starts_with("> ASSISTANT:") {
                    if !current_text.trim().is_empty() {
                        messages.push(ChatMessage {
                            role: current_role,
                            content: current_text.clone(),
                            timestamp: None,
                            full_content: None,
                            source_path: None,
                        });
                    }
                    current_role = "assistant".to_string();
                    current_text = String::new();
                } else {
                    current_text.push_str(line);
                    current_text.push('\n');
                }
            }
            if !current_text.trim().is_empty() {
                messages.push(ChatMessage {
                    role: current_role,
                    content: current_text,
                    timestamp: None,
                    full_content: None,
                    source_path: None,
                });
            }
            return Ok(messages);
        }

        // Claude / Codex 的 JSONL 逻辑
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(msg_arr) = val.get("messages").and_then(|v| v.as_array()) {
                    for m in msg_arr {
                        messages.push(Self::parse_message(m));
                    }
                } else if val.get("type").and_then(|v| v.as_str()) == Some("response_item") {
                    let payload = &val["payload"];
                    if payload.get("type").and_then(|v| v.as_str()) == Some("message") {
                        messages.push(Self::parse_message(payload));
                    }
                } else if val.get("type").and_then(|v| v.as_str()) == Some("event_msg")
                    && val.pointer("/payload/type").and_then(|v| v.as_str()) == Some("user_message")
                {
                    if let Some(text) = val.pointer("/payload/message").and_then(|v| v.as_str()) {
                        messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: text.to_string(),
                            timestamp: None,
                            full_content: None,
                            source_path: None,
                        });
                    }
                } else if val.get("role").is_some() {
                    messages.push(Self::parse_message(&val));
                }
            }
        }

        Ok(messages)
    }

    fn parse_gemini_session_messages(
        val: &serde_json::Value,
        session_path: &Path,
    ) -> Vec<ChatMessage> {
        let Some(messages) = val.get("messages").and_then(|item| item.as_array()) else {
            return Vec::new();
        };

        let session_id = val
            .get("sessionId")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let workspace_dir = session_path
            .parent()
            .and_then(|path| path.parent())
            .map(PathBuf::from);
        let tool_output_dir = workspace_dir
            .as_ref()
            .map(|path| path.join("tool-outputs").join(format!("session-{}", session_id)));

        let mut parsed = Vec::<ChatMessage>::new();
        for item in messages {
            let raw_role = item
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or("system");
            let role = match raw_role {
                "user" => "user",
                "gemini" => "assistant",
                "model" => "assistant",
                "tool" => "tool",
                _ => "system",
            }
            .to_string();
            let message_timestamp = item
                .get("timestamp")
                .and_then(|value| value.as_str())
                .and_then(parse_rfc3339_seconds);

            let mut sections = Vec::new();
            let mut full_sections = Vec::new();
            if let Some(main_text) = Self::extract_gemini_message_text(item) {
                sections.push(main_text.clone());
                full_sections.push(main_text);
            }

            if let Some(thoughts) = item.get("thoughts").and_then(|value| value.as_array()) {
                let thought_lines = thoughts
                    .iter()
                    .filter_map(|thought| {
                        let subject = thought.get("subject").and_then(|value| value.as_str())?;
                        let description =
                            thought.get("description").and_then(|value| value.as_str()).unwrap_or("");
                        Some(if description.trim().is_empty() {
                            format!("- {}", subject.trim())
                        } else {
                            format!(
                                "- {}: {}",
                                subject.trim(),
                                truncate_message_block(description.trim(), 360)
                            )
                        })
                    })
                    .take(3)
                    .collect::<Vec<_>>();
                if !thought_lines.is_empty() {
                    let thought_block = format!("[思路摘要]\n{}", thought_lines.join("\n"));
                    sections.push(thought_block.clone());
                    full_sections.push(thought_block);
                }
            }

            let content = sections.join("\n\n");
            if !content.trim().is_empty() {
                let full_content = full_sections.join("\n\n");
                parsed.push(ChatMessage {
                    role: role.clone(),
                    content: content.clone(),
                    timestamp: message_timestamp,
                    full_content: (full_content != content).then_some(full_content),
                    source_path: None,
                });
            }

            if let Some(tool_calls) = item.get("toolCalls").and_then(|value| value.as_array()) {
                for tool in tool_calls {
                    if let Some(formatted_tool) =
                        Self::format_gemini_tool_call(tool, tool_output_dir.as_deref())
                    {
                        parsed.push(ChatMessage {
                            role: "tool".to_string(),
                            content: formatted_tool.preview.clone(),
                            timestamp: formatted_tool.timestamp.or(message_timestamp),
                            full_content: formatted_tool.full_content,
                            source_path: formatted_tool.source_path,
                        });
                    }
                }
            }
        }

        if let Some(workspace_dir) = workspace_dir {
            let logs_path = workspace_dir.join("logs.json");
            if let Ok(raw_logs) = fs::read_to_string(&logs_path) {
                if let Ok(log_value) = serde_json::from_str::<serde_json::Value>(&raw_logs) {
                    if let Some(items) = log_value.as_array() {
                        let related = items
                            .iter()
                            .filter(|item| item.get("sessionId").and_then(|value| value.as_str()) == Some(session_id.as_str()))
                            .collect::<Vec<_>>();
                        if !related.is_empty() {
                            let mut inserted_count = 0usize;
                            for item in &related {
                                let Some(message) =
                                    item.get("message").and_then(|value| value.as_str()).map(|value| value.trim())
                                else {
                                    continue;
                                };
                                if message.is_empty() {
                                    continue;
                                }
                                let timestamp = item
                                    .get("timestamp")
                                    .and_then(|value| value.as_str())
                                    .and_then(parse_rfc3339_seconds);
                                let msg_type = item
                                    .get("type")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or("unknown");

                                let duplicated = parsed.iter().any(|existing| {
                                    let same_role = matches!(msg_type, "user") && existing.role == "user";
                                    let same_time = timestamp.is_some() && existing.timestamp == timestamp;
                                    same_role
                                        && same_time
                                        && normalize_message_for_compare(&existing.content)
                                            == normalize_message_for_compare(message)
                                });

                                if duplicated {
                                    continue;
                                }

                                parsed.push(ChatMessage {
                                    role: if msg_type.eq_ignore_ascii_case("user") {
                                        "system".to_string()
                                    } else {
                                        "tool".to_string()
                                    },
                                    content: format!(
                                        "[日志事件]\n类型：{}\n内容：{}",
                                        msg_type,
                                        truncate_message_block(message, 220)
                                    ),
                                    timestamp,
                                    full_content: Some(message.to_string()),
                                    source_path: Some(logs_path.to_string_lossy().into_owned()),
                                });
                                inserted_count += 1;
                            }

                            let summary_lines = related
                                .iter()
                                .take(8)
                                .filter_map(|item| {
                                    let msg_type = item.get("type").and_then(|value| value.as_str()).unwrap_or("unknown");
                                    let message = item.get("message").and_then(|value| value.as_str()).unwrap_or("");
                                    let timestamp = item.get("timestamp").and_then(|value| value.as_str()).unwrap_or("");
                                    Some(format!(
                                        "- [{}] {} {}",
                                        msg_type,
                                        timestamp,
                                        truncate_message_block(message, 120)
                                    ))
                                })
                                .collect::<Vec<_>>();

                            parsed.push(ChatMessage {
                                role: "system".to_string(),
                                content: format!(
                                    "Gemini logs.json 共记录到当前会话 {} 条事件；其中 {} 条已按时间轴并入消息流，其余因与现有转录重复而跳过。\n\n{}",
                                    related.len(),
                                    inserted_count,
                                    summary_lines.join("\n")
                                ),
                                timestamp: related
                                    .last()
                                    .and_then(|item| item.get("timestamp").and_then(|value| value.as_str()))
                                    .and_then(parse_rfc3339_seconds),
                                full_content: Some(
                                    related
                                        .iter()
                                        .map(|item| item.to_string())
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                ),
                                source_path: Some(logs_path.to_string_lossy().into_owned()),
                            });
                        }
                    }
                }
            }

            if let Some(tool_output_dir) = tool_output_dir {
                if tool_output_dir.exists() {
                    let output_files = fs::read_dir(&tool_output_dir)
                        .ok()
                        .into_iter()
                        .flat_map(|entries| entries.flatten())
                        .filter_map(|entry| {
                            let path = entry.path();
                            if path.is_file() {
                                path.file_name().map(|name| name.to_string_lossy().into_owned())
                            } else {
                                None
                            }
                        })
                        .take(12)
                        .collect::<Vec<_>>();
                    if !output_files.is_empty() {
                        parsed.push(ChatMessage {
                            role: "system".to_string(),
                            content: format!(
                                "当前会话的工具输出目录：{}\n\n{}",
                                tool_output_dir.to_string_lossy(),
                                output_files
                                    .iter()
                                    .map(|item| format!("- {}", item))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            ),
                            timestamp: None,
                            full_content: None,
                            source_path: Some(tool_output_dir.to_string_lossy().into_owned()),
                        });
                    }
                }
            }
        }

        sort_chat_messages_by_timeline(&mut parsed);
        parsed
    }

    fn format_gemini_tool_call(
        tool: &serde_json::Value,
        tool_output_dir: Option<&Path>,
    ) -> Option<FormattedToolCall> {
        let name = tool.get("name").and_then(|value| value.as_str())?;
        let status = tool.get("status").and_then(|value| value.as_str()).unwrap_or("unknown");
        let description = tool
            .get("description")
            .and_then(|value| value.as_str())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        let timestamp = tool
            .get("timestamp")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let args = tool
            .get("args")
            .map(|value| truncate_message_block(&value.to_string(), 260))
            .unwrap_or_default();
        let result_display = tool
            .get("resultDisplay")
            .map(|value| truncate_message_block(&flatten_json_preview(value), 320))
            .filter(|value| !value.trim().is_empty());
        let preview = Self::extract_gemini_tool_result_preview(tool, tool_output_dir);

        let mut lines = vec![format!("{} [{}] {}", name, status, timestamp).trim().to_string()];
        if let Some(description) = description {
            lines.push(format!("说明：{}", description));
        }
        if !args.trim().is_empty() && args != "{}" {
            lines.push(format!("参数：{}", args));
        }
        if let Some(result_display) = result_display {
            lines.push(format!("结果：{}", result_display));
        }
        let source_path = preview.as_ref().and_then(|item| item.source_path.clone());
        if let Some(preview) = preview.as_ref().map(|item| item.preview.clone()) {
            lines.push(format!("输出预览：{}", preview));
        }
        Some(FormattedToolCall {
            preview: lines.join("\n"),
            full_content: preview.as_ref().and_then(|item| item.full_content.clone()).map(|full| {
                let mut full_lines = lines
                    .iter()
                    .filter(|line| !line.starts_with("输出预览："))
                    .cloned()
                    .collect::<Vec<_>>();
                full_lines.push(format!("完整输出：{}", full));
                full_lines.join("\n")
            }),
            source_path,
            timestamp: tool
                .get("timestamp")
                .and_then(|value| value.as_str())
                .and_then(parse_rfc3339_seconds),
        })
    }

    fn extract_gemini_tool_result_preview(
        tool: &serde_json::Value,
        tool_output_dir: Option<&Path>,
    ) -> Option<FormattedToolCall> {
        let result = tool.get("result")?.as_array()?;
        for item in result {
            let output = item
                .pointer("/functionResponse/response/output")
                .and_then(|value| value.as_str())
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            let Some(output) = output else {
                continue;
            };

            if let Some(file_path) = extract_full_output_path(output) {
                if let Ok(raw) = fs::read_to_string(&file_path) {
                    let preview = truncate_message_block(raw.trim(), 400);
                    if !preview.is_empty() {
                        return Some(FormattedToolCall {
                            preview: format!("{} ({})", preview, file_path),
                            full_content: Some(raw.trim().to_string()),
                            source_path: Some(file_path),
                            timestamp: None,
                        });
                    }
                }
            }

            let cleaned = output
                .replace("<tool_output_masked>", "")
                .replace("</tool_output_masked>", "")
                .trim()
                .to_string();
            if !cleaned.is_empty() {
                return Some(FormattedToolCall {
                    preview: truncate_message_block(&cleaned, 400),
                    full_content: Some(cleaned),
                    source_path: None,
                    timestamp: None,
                });
            }
        }

        if let Some(dir) = tool_output_dir {
            let tool_id = tool.get("id").and_then(|value| value.as_str()).unwrap_or_default();
            if !tool_id.is_empty() {
                let prefix = tool_id.replace(':', "_");
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let file_name = path
                            .file_name()
                            .map(|value| value.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        if path.is_file() && file_name.contains(&prefix) {
                            if let Ok(raw) = fs::read_to_string(&path) {
                                let preview = truncate_message_block(raw.trim(), 320);
                                if !preview.is_empty() {
                                    return Some(FormattedToolCall {
                                        preview: format!("{} ({})", preview, path.to_string_lossy()),
                                        full_content: Some(raw.trim().to_string()),
                                        source_path: Some(path.to_string_lossy().into_owned()),
                                        timestamp: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_gemini_message_text(val: &serde_json::Value) -> Option<String> {
        if let Some(text) = val.get("content").and_then(|value| value.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }

        if let Some(items) = val.get("content").and_then(|value| value.as_array()) {
            let mut chunks = Vec::new();
            for item in items {
                if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        chunks.push(trimmed.to_string());
                    }
                }
            }
            if !chunks.is_empty() {
                return Some(chunks.join("\n"));
            }
        }

        None
    }

    fn parse_message(val: &serde_json::Value) -> ChatMessage {
        let role = val
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let text_content = if let Some(content_arr) = val.get("content").and_then(|v| v.as_array())
        {
            let mut text = String::new();
            for c in content_arr {
                if let Some(t) = c.get("text").and_then(|t| t.as_str()) {
                    text.push_str(t);
                    text.push('\n');
                } else {
                    // maybe tool use
                    text.push_str(&format!("🔧 [Tool Action]: {}\n", c.to_string()));
                }
            }
            text
        } else if let Some(text) = val.get("content").and_then(|v| v.as_str()) {
            text.to_string()
        } else {
            val.to_string()
        };

        ChatMessage {
            role,
            content: text_content,
            timestamp: None,
            full_content: None,
            source_path: None,
        }
    }

    fn extract_text_from_content(content: Option<&serde_json::Value>) -> Option<String> {
        let arr = content?.as_array()?;
        let mut text = String::new();
        for item in arr {
            if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
            if let Some(t) = item.get("input_text").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
        }
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn sanitize_session_title(text: String) -> Option<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }

        let first_line = trimmed.lines().find(|line| !line.trim().is_empty())?.trim();
        let skip_prefixes = [
            "<environment_context>",
            "<local-command-caveat>",
            "<command-name>",
            "<local-command-stdout>",
        ];
        if skip_prefixes
            .iter()
            .any(|prefix| first_line.starts_with(prefix))
        {
            return None;
        }

        Some(first_line.chars().take(48).collect())
    }

    fn format_index_timestamp(updated_at: i64) -> String {
        chrono::DateTime::<Utc>::from_timestamp(updated_at, 0)
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    fn repair_codex_session_index_for_instance(
        instance: &CodexInstanceRef,
    ) -> Result<(usize, usize, usize), String> {
        let db_path = instance.data_dir.join("state_5.sqlite");
        let index_path = instance.data_dir.join("session_index.jsonl");

        if !db_path.exists() {
            return Ok((0, 0, 0));
        }

        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("打开 Codex 数据库失败 ({}): {}", db_path.display(), e))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, title, first_user_message, updated_at
             FROM threads
             WHERE archived = 0
             ORDER BY updated_at DESC",
            )
            .map_err(|e| format!("读取 Codex 线程失败 ({}): {}", instance.name, e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| format!("查询 Codex 线程失败 ({}): {}", instance.name, e))?;

        let mut thread_map = HashMap::<String, JsonValue>::new();
        for row in rows.flatten() {
            let (id, title, first_user_message, updated_at) = row;
            let final_title = Self::sanitize_session_title(title)
                .or_else(|| Self::sanitize_session_title(first_user_message))
                .unwrap_or_else(|| id.clone());
            thread_map.insert(
                id.clone(),
                serde_json::json!({
                    "id": id,
                    "thread_name": final_title,
                    "updated_at": Self::format_index_timestamp(updated_at),
                }),
            );
        }

        let existing_content = fs::read_to_string(&index_path).unwrap_or_default();
        let mut existing_map = HashMap::<String, JsonValue>::new();
        for line in existing_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<JsonValue>(trimmed) else {
                continue;
            };
            let Some(id) = value.get("id").and_then(|item| item.as_str()) else {
                continue;
            };
            existing_map.insert(id.to_string(), value);
        }

        let thread_count = thread_map.len();
        let added_index_entries = thread_map
            .keys()
            .filter(|id| !existing_map.contains_key(*id))
            .count();
        let removed_stale_entries = existing_map
            .keys()
            .filter(|id| !thread_map.contains_key(*id))
            .count();

        let mut merged = thread_map.into_iter().collect::<Vec<_>>();
        merged.sort_by(|a, b| a.0.cmp(&b.0));
        let final_content = merged
            .into_iter()
            .map(|(_, value)| serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建 Codex 索引目录失败 ({}): {}", parent.display(), e))?;
        }
        fs::write(
            &index_path,
            if final_content.is_empty() {
                String::new()
            } else {
                format!("{}\n", final_content)
            },
        )
        .map_err(|e| {
            format!(
                "写入 Codex session_index.jsonl 失败 ({}): {}",
                index_path.display(),
                e
            )
        })?;

        Ok((thread_count, added_index_entries, removed_stale_entries))
    }

    fn sync_missing_codex_threads_to_instance(
        target: &CodexInstanceRef,
        snapshots: &[ThreadSnapshot],
    ) -> Result<PathBuf, String> {
        let backup_dir = Self::backup_codex_instance_files(&target.data_dir)?;
        let existing_index_ids =
            Self::read_codex_session_index_map(&target.data_dir.join("session_index.jsonl"))
                .keys()
                .cloned()
                .collect::<HashSet<_>>();
        let db_path = target.data_dir.join("state_5.sqlite");
        let mut connection = Connection::open(&db_path)
            .map_err(|e| format!("打开目标实例数据库失败 ({}): {}", target.name, e))?;
        let target_columns = Self::read_thread_columns(&connection)?;
        let transaction = connection
            .transaction()
            .map_err(|e| format!("开启目标实例事务失败 ({}): {}", target.name, e))?;

        for snapshot in snapshots {
            let target_rollout_path = Self::copy_codex_rollout_file(snapshot, &target.data_dir)?;
            let mut row_data = snapshot.row_data.clone();
            row_data.set_text(
                "rollout_path",
                target_rollout_path.to_string_lossy().to_string(),
            );
            Self::insert_codex_thread_row(&transaction, &target_columns, &row_data)?;
        }

        transaction
            .commit()
            .map_err(|e| format!("提交目标实例事务失败 ({}): {}", target.name, e))?;

        Self::append_codex_session_index_entries(
            &target.data_dir.join("session_index.jsonl"),
            &existing_index_ids,
            snapshots,
        )?;
        Ok(backup_dir)
    }

    fn backup_codex_instance_files(data_dir: &PathBuf) -> Result<PathBuf, String> {
        let backup_dir = data_dir.join(format!(
            "backup-{}-instance-thread-sync",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("创建备份目录失败 ({}): {}", data_dir.display(), e))?;

        for file_name in ["state_5.sqlite", "session_index.jsonl"] {
            let source = data_dir.join(file_name);
            if !source.exists() {
                continue;
            }
            let target = backup_dir.join(format!("{}.bak", file_name));
            fs::copy(&source, &target).map_err(|e| {
                format!(
                    "备份文件失败 ({} -> {}): {}",
                    source.display(),
                    target.display(),
                    e
                )
            })?;
        }

        Ok(backup_dir)
    }

    fn copy_codex_rollout_file(
        snapshot: &ThreadSnapshot,
        target_root: &PathBuf,
    ) -> Result<PathBuf, String> {
        let relative = snapshot
            .rollout_path
            .strip_prefix(&snapshot.source_root)
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|_| {
                PathBuf::from("sessions").join(
                    snapshot
                        .rollout_path
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("rollout.jsonl")),
                )
            });
        let target_path = target_root.join(relative);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目标线程目录失败 ({}): {}", parent.display(), e))?;
        }
        if !target_path.exists() {
            fs::copy(&snapshot.rollout_path, &target_path).map_err(|e| {
                format!(
                    "复制线程文件失败 ({} -> {}): {}",
                    snapshot.rollout_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        }
        Ok(target_path)
    }

    fn insert_codex_thread_row(
        transaction: &Transaction<'_>,
        target_columns: &[String],
        row_data: &ThreadRowData,
    ) -> Result<(), String> {
        let mut values = Vec::with_capacity(target_columns.len());
        for column in target_columns {
            values.push(row_data.get_value(column).cloned().unwrap_or(Value::Null));
        }

        let quoted_columns = target_columns
            .iter()
            .map(|column| Self::quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = std::iter::repeat("?")
            .take(target_columns.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT OR IGNORE INTO threads ({}) VALUES ({})",
            quoted_columns, placeholders
        );

        transaction
            .execute(&query, params_from_iter(values.iter()))
            .map_err(|e| format!("写入目标线程记录失败: {}", e))?;
        Ok(())
    }

    fn append_codex_session_index_entries(
        index_path: &PathBuf,
        existing_ids: &HashSet<String>,
        snapshots: &[ThreadSnapshot],
    ) -> Result<(), String> {
        let mut lines = Vec::new();
        for snapshot in snapshots {
            if existing_ids.contains(&snapshot.id) {
                continue;
            }
            lines.push(
                serde_json::to_string(&snapshot.session_index_entry)
                    .map_err(|e| format!("序列化 session_index 条目失败: {}", e))?,
            );
        }
        if lines.is_empty() {
            return Ok(());
        }

        let needs_prefix = index_path.exists() && !Self::file_ends_with_newline(index_path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(index_path)
            .map_err(|e| {
                format!(
                    "打开 session_index.jsonl 失败 ({}): {}",
                    index_path.display(),
                    e
                )
            })?;

        use std::io::Write;
        if needs_prefix {
            file.write_all(b"\n").map_err(|e| {
                format!(
                    "写入 session_index 换行失败 ({}): {}",
                    index_path.display(),
                    e
                )
            })?;
        }
        for line in lines {
            file.write_all(line.as_bytes())
                .and_then(|_| file.write_all(b"\n"))
                .map_err(|e| {
                    format!(
                        "追加 session_index 条目失败 ({}): {}",
                        index_path.display(),
                        e
                    )
                })?;
        }
        Ok(())
    }

fn file_ends_with_newline(path: &PathBuf) -> Result<bool, String> {
        let bytes =
            fs::read(path).map_err(|e| format!("读取文件失败 ({}): {}", path.display(), e))?;
        Ok(bytes.last().copied() == Some(b'\n'))
    }
}

fn is_gemini_history_preview_candidate(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "json" | "jsonl" | "log" | "md" | "txt")
        || name.contains("history")
        || name.contains("session")
        || name.contains("chat")
}

fn parse_rfc3339_seconds(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|item| item.timestamp().try_into().ok())
}

fn file_timestamp_seconds(path: &Path, prefer_modified: bool) -> u64 {
    let metadata = fs::metadata(path).ok();
    let system_time = if prefer_modified {
        metadata.as_ref().and_then(|item| item.modified().ok())
    } else {
        metadata.as_ref().and_then(|item| item.created().ok())
    }
    .or_else(|| metadata.as_ref().and_then(|item| item.modified().ok()))
    .unwrap_or(SystemTime::UNIX_EPOCH);

    system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn truncate_single_line(text: &str, max_chars: usize) -> String {
    let normalized = text.replace(['\r', '\n'], " ");
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        compact.chars().take(max_chars).collect::<String>() + "..."
    }
}

fn truncate_message_block(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        trimmed.chars().take(max_chars).collect::<String>() + "..."
    }
}

fn normalize_message_for_compare(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn flatten_json_preview(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn extract_full_output_path(text: &str) -> Option<String> {
    let marker = "Full output available at:";
    let start = text.find(marker)? + marker.len();
    let tail = text[start..].trim();
    let line = tail.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

fn sort_chat_messages_by_timeline(messages: &mut [ChatMessage]) {
    messages.sort_by(|a, b| match (a.timestamp, b.timestamp) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}
