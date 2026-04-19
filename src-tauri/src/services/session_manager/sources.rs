use super::{ChatSession, SessionManager};
use super::parsing::{file_timestamp_seconds, parse_rfc3339_seconds, truncate_single_line};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

impl SessionManager {
    pub(super) fn collect_gemini_tmp_sessions(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
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
                    .flat_map(|item| {
                        item.get("toolCalls")
                            .and_then(|value| value.as_array())
                            .into_iter()
                            .flatten()
                    })
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

    pub(super) fn collect_gemini_workspace_history(
        home_dir: &PathBuf,
        sessions: &mut Vec<ChatSession>,
    ) {
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

    pub(super) fn collect_claude_sessions(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
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
}
