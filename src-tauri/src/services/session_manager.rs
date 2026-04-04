use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{PathBuf, Path};
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages_count: usize,
    pub filepath: String,
}

pub struct SessionManager;

impl SessionManager {
    /// 全域进程雷达：探测环境内存活的第三方 AI CLI 工具
    pub fn scan_zombie_processes() -> Vec<ZombieProcess> {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut zombies = Vec::new();

        for (pid, process) in sys.processes() {
            let cmd_args: Vec<String> = process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect();
            let cmd = cmd_args.join(" ").to_lowercase();
            let name = process.name().to_string_lossy().to_string().to_lowercase();
            let cwd = process.cwd().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            
            let mut tool_type = None;

            if cmd.contains("claude-cli") || (cmd.contains("claude") && cmd.contains("node")) {
                tool_type = Some("ClaudeCode");
            } else if cmd.contains("aider") && !cmd.contains("cargo") {
                tool_type = Some("Aider");
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
        let projects_dir = home_dir.join(".claude").join("projects");
        
        if !projects_dir.exists() {
            return Ok(vec![]);
        }

        let mut sessions = Vec::new();
        // 因为 claude/projects 是一层 hash 目录，比如 1a2b3c/, 里面有 *.jsonl
        if let Ok(entries) = fs::read_dir(projects_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let proj_path = entry.path();
                    // 这里我们取该目录名的 hash 前 8 位作为显示名字
                    let safe_title = entry.file_name().to_string_lossy().to_string();
                    let title = if safe_title.len() > 8 {
                        safe_title[..8].to_string()
                    } else {
                        safe_title
                    };

                    if let Ok(files) = fs::read_dir(proj_path) {
                        for file_entry in files.flatten() {
                            let path = file_entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                                if let Ok(metadata) = fs::metadata(&path) {
                                    let modified = metadata.modified()
                                        .unwrap_or(SystemTime::UNIX_EPOCH)
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                        
                                    // 仅统计实际有内容的行数
                                    let content = fs::read_to_string(&path).unwrap_or_default();
                                    let count = content.lines().count();
                                        
                                    sessions.push(ChatSession {
                                        id: path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
                                        title: format!("Project-{}", title),
                                        created_at: modified, // 粗略值
                                        updated_at: modified,
                                        messages_count: count,
                                        filepath: path.to_string_lossy().into_owned(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        // 动态侦测进程并吸血：通过僵尸雷达拉取 Aider 缓存
        let zombies = Self::scan_zombie_processes();
        for zombie in zombies {
            if zombie.tool_type == "Aider" && !zombie.cwd.is_empty() {
                let path = PathBuf::from(&zombie.cwd).join(".aider.chat.history.md");
                if path.exists() {
                    if let Ok(metadata) = fs::metadata(&path) {
                        let modified = metadata.modified()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        let count = content.lines().count();
                        
                        let project_name = PathBuf::from(&zombie.cwd).file_name().unwrap_or_default().to_string_lossy().into_owned();
                        
                        // 防止重复推入同一个项目的 Aider 会话（多个相同 cwd 的 aider 进程）
                        if !sessions.iter().any(|s| s.filepath == path.to_string_lossy().to_string()) {
                            sessions.push(ChatSession {
                                id: format!("aider-{}", zombie.pid),
                                title: format!("Aider // {}", project_name),
                                created_at: modified,
                                updated_at: modified,
                                messages_count: count, // markdown lines count
                                filepath: path.to_string_lossy().into_owned(),
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

    /// 获取单个 jsonl 文件的聊天对话
    pub fn get_session_details(filepath: &str) -> Result<Vec<ChatMessage>, String> {
        let path = PathBuf::from(filepath);
        if !path.exists() {
            return Err("File not found".into());
        }
        
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut messages = Vec::new();
        
        // 兼容 Aider 离线 Markdown 格式
        if filepath.ends_with(".md") {
            // Aider 的聊天记录通常是很长的 Markdown，如果没有明显结构，我们可以把它作为一个超级 Block 返回
            // 或者用正则切割 USER 和 ASSISTANT 块。由于 Aider 常以 `> USER:` 和 `> ASSISTANT:` 作为交互分隔。
            let mut current_role = "system".to_string();
            let mut current_text = String::new();
            
            for line in content.lines() {
                if line.starts_with("> USER:") {
                    if !current_text.trim().is_empty() {
                        messages.push(ChatMessage { role: current_role, content: current_text.clone(), timestamp: None });
                    }
                    current_role = "user".to_string();
                    current_text = String::new();
                } else if line.starts_with("> ASSISTANT:") {
                    if !current_text.trim().is_empty() {
                        messages.push(ChatMessage { role: current_role, content: current_text.clone(), timestamp: None });
                    }
                    current_role = "assistant".to_string();
                    current_text = String::new();
                } else {
                    current_text.push_str(line);
                    current_text.push('\n');
                }
            }
            if !current_text.trim().is_empty() {
                messages.push(ChatMessage { role: current_role, content: current_text, timestamp: None });
            }
            return Ok(messages);
        }

        // Claude 的 JSONL 逻辑
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                // Claude CLI 可能会混有 {"messages": [{"role": "user", "content": [...]}]}
                // 也可能单行即为 {"role":"...", "content":"..."}
                
                // 为了兼容性，我们只尝试提取 role 和 content
                // 1. 若是一个完整的 payload 发起请求
                if let Some(msg_arr) = val.get("messages").and_then(|v| v.as_array()) {
                    for m in msg_arr {
                        messages.push(Self::parse_message(m));
                    }
                } else if val.get("role").is_some() {
                    messages.push(Self::parse_message(&val));
                }
            }
        }
        
        Ok(messages)
    }

    fn parse_message(val: &serde_json::Value) -> ChatMessage {
        let role = val.get("role").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        
        let text_content = if let Some(content_arr) = val.get("content").and_then(|v| v.as_array()) {
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
        }
    }
}
