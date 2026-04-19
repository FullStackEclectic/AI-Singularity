use super::{ChatMessage, FormattedToolCall, SessionManager};
use chrono::DateTime;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

impl SessionManager {
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
                        child
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
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
                if value.get("sessionId").is_some()
                    && value
                        .get("messages")
                        .and_then(|item| item.as_array())
                        .is_some()
                {
                    return Ok(Self::parse_gemini_session_messages(&value, &path));
                }
            }
        }

        if filepath.ends_with(".md") {
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
        let tool_output_dir = workspace_dir.as_ref().map(|path| {
            path.join("tool-outputs")
                .join(format!("session-{}", session_id))
        });

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
                        let description = thought
                            .get("description")
                            .and_then(|value| value.as_str())
                            .unwrap_or("");
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
                            .filter(|item| {
                                item.get("sessionId").and_then(|value| value.as_str())
                                    == Some(session_id.as_str())
                            })
                            .collect::<Vec<_>>();
                        if !related.is_empty() {
                            let mut inserted_count = 0usize;
                            for item in &related {
                                let Some(message) = item
                                    .get("message")
                                    .and_then(|value| value.as_str())
                                    .map(|value| value.trim())
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
                                    let same_role =
                                        matches!(msg_type, "user") && existing.role == "user";
                                    let same_time =
                                        timestamp.is_some() && existing.timestamp == timestamp;
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
                                    let msg_type = item
                                        .get("type")
                                        .and_then(|value| value.as_str())
                                        .unwrap_or("unknown");
                                    let message = item
                                        .get("message")
                                        .and_then(|value| value.as_str())
                                        .unwrap_or("");
                                    let timestamp = item
                                        .get("timestamp")
                                        .and_then(|value| value.as_str())
                                        .unwrap_or("");
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
                                    .and_then(|item| {
                                        item.get("timestamp").and_then(|value| value.as_str())
                                    })
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
                                path.file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
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

    pub(super) fn extract_gemini_message_text(val: &serde_json::Value) -> Option<String> {
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

    fn format_gemini_tool_call(
        tool: &serde_json::Value,
        tool_output_dir: Option<&Path>,
    ) -> Option<FormattedToolCall> {
        let name = tool.get("name").and_then(|value| value.as_str())?;
        let status = tool
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
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

        let mut lines = vec![format!("{} [{}] {}", name, status, timestamp)
            .trim()
            .to_string()];
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
            full_content: preview
                .as_ref()
                .and_then(|item| item.full_content.clone())
                .map(|full| {
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
            let tool_id = tool
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
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
                                        preview: format!(
                                            "{} ({})",
                                            preview,
                                            path.to_string_lossy()
                                        ),
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
                    text.push_str(&format!("🔧 [Tool Action]: {}\n", c));
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

    pub(super) fn extract_text_from_content(content: Option<&serde_json::Value>) -> Option<String> {
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

    pub(super) fn sanitize_session_title(text: String) -> Option<String> {
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

pub(super) fn parse_rfc3339_seconds(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|item| item.timestamp().try_into().ok())
}

pub(super) fn file_timestamp_seconds(path: &Path, prefer_modified: bool) -> u64 {
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

pub(super) fn truncate_single_line(text: &str, max_chars: usize) -> String {
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
