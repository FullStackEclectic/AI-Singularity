use super::{SessionManager, SessionTrashSummary};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

impl SessionManager {
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
}
