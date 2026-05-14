use crate::db::Database;
use crate::error::AppResult;
use crate::models::PromptConfig;
use rusqlite::params;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

pub struct PromptService<'a> {
    db: &'a Database,
}

impl<'a> PromptService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_prompts(&self) -> AppResult<Vec<PromptConfig>> {
        let sql = "SELECT id, name, description, target_file, content, is_active, tool_targets, created_at, updated_at FROM prompts";
        self.db
            .query_rows(sql, &[], |row| {
                let desc: Option<String> = row.get(2)?;
                let tool_targets: Option<String> = row.get(6)?;
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;
                Ok(PromptConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: desc,
                    target_file: row.get(3)?,
                    content: row.get(4)?,
                    is_active: row.get::<_, i32>(5)? != 0,
                    tool_targets,
                    created_at: created_at_str.parse().unwrap_or_default(),
                    updated_at: updated_at_str.parse().unwrap_or_default(),
                })
            })
            .map_err(Into::into)
    }

    pub fn save_prompt(&self, prompt: PromptConfig) -> AppResult<()> {
        let sql = "INSERT OR REPLACE INTO prompts (id, name, description, target_file, content, is_active, tool_targets, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
        self.db.execute(
            sql,
            rusqlite::params![
                prompt.id,
                prompt.name,
                prompt.description,
                prompt.target_file,
                prompt.content,
                if prompt.is_active { 1 } else { 0 },
                prompt.tool_targets,
                prompt.created_at.to_rfc3339(),
                prompt.updated_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn delete_prompt(&self, id: &str) -> AppResult<()> {
        self.db
            .execute("DELETE FROM prompts WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Sync the prompt to a specific directory
    pub fn sync_prompt_to_workspace(&self, id: &str, workspace_dir: &str) -> AppResult<()> {
        // fetch the prompt
        let prompt = self.list_prompts()?.into_iter().find(|p| p.id == id);
        if let Some(p) = prompt {
            let path = PathBuf::from(workspace_dir).join(&p.target_file);
            match fs::write(&path, &p.content) {
                Ok(_) => {
                    info!("Successfully synced prompt to {}", path.display());
                }
                Err(e) => {
                    error!("Failed to sync prompt to {}: {}", path.display(), e);
                    return Err(e.into());
                }
            }
        } else {
            return Err(anyhow::anyhow!("Prompt not found").into());
        }
        Ok(())
    }

    /// 根据 tool_targets 字段自动分发 Prompt 到各工具的默认配置文件
    /// - "claude" → ~/.claude.md
    /// - "aider"  → ~/.aider.conf.yml (system-prompt 字段)
    pub fn sync_to_tool_defaults(&self, id: &str) -> AppResult<Vec<String>> {
        let prompt = self
            .list_prompts()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Prompt not found"))?;

        let targets_raw = prompt.tool_targets.as_deref().unwrap_or("");
        let targets: Vec<&str> = targets_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if targets.is_empty() {
            return Err(anyhow::anyhow!(
                "该 Prompt 未设置 tool_targets，请先在编辑页面指定目标工具后再分发"
            )
            .into());
        }

        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("无法获取用户 Home 目录"))?;
        let mut synced = Vec::new();

        for target in targets {
            match target.to_lowercase().as_str() {
                "claude" => {
                    let path = home.join(".claude.md");
                    match fs::write(&path, &prompt.content) {
                        Ok(_) => {
                            info!(
                                "✅ Prompt '{}' 已分发至 Claude Code: {:?}",
                                prompt.name, path
                            );
                            synced.push(format!("Claude Code: {}", path.display()));
                        }
                        Err(e) => {
                            error!("❌ 写入 Claude Code 配置失败: {}", e);
                            return Err(e.into());
                        }
                    }
                }
                "aider" => {
                    let path = home.join(".aider.conf.yml");
                    // 读取现有 YAML 内容，仅替换/插入 system-prompt 字段
                    let existing = fs::read_to_string(&path).unwrap_or_default();
                    let new_content = Self::patch_aider_system_prompt(&existing, &prompt.content);
                    match fs::write(&path, new_content) {
                        Ok(_) => {
                            info!("✅ Prompt '{}' 已分发至 Aider: {:?}", prompt.name, path);
                            synced.push(format!("Aider: {}", path.display()));
                        }
                        Err(e) => {
                            error!("❌ 写入 Aider 配置失败: {}", e);
                            return Err(e.into());
                        }
                    }
                }
                unknown => {
                    warn!("未知 tool_target: '{}', 跳过", unknown);
                }
            }
        }

        if synced.is_empty() {
            return Err(
                anyhow::anyhow!("没有任何已知工具被成功分发，请检查 tool_targets 设置").into(),
            );
        }

        Ok(synced)
    }

    /// 将 system_prompt 内容注入或替换 .aider.conf.yml 中的对应字段
    /// 遵循 YAML 格式：`system-prompt: |\n  <内容>`
    fn patch_aider_system_prompt(existing_yaml: &str, new_prompt: &str) -> String {
        let marker = "system-prompt:";
        // 将 prompt 内容转为 yaml block scalar（每行前加两个空格）
        let indented: String = new_prompt
            .lines()
            .map(|l| format!("  {}", l))
            .collect::<Vec<_>>()
            .join("\n");
        let new_block = format!("{}|\n{}", marker, indented);

        // 如果已存在 system-prompt 字段，替换整个块
        if let Some(start) = existing_yaml.find(marker) {
            // 找到下一个顶级 key（非空格开头的行）或文件结尾
            let after_marker = &existing_yaml[start + marker.len()..];
            let block_end = after_marker
                .lines()
                .skip(1) // 跳过 marker 行自身
                .enumerate()
                .find(|(_, line)| !line.is_empty() && !line.starts_with(' '))
                .map(|(i, _)| {
                    // 计算字节偏移
                    after_marker
                        .lines()
                        .take(i + 1)
                        .map(|l| l.len() + 1)
                        .sum::<usize>()
                })
                .unwrap_or(after_marker.len());

            let before = &existing_yaml[..start];
            let after = &existing_yaml[start + marker.len() + block_end..];
            format!("{}{}{}", before, new_block, after)
        } else {
            // 不存在则追加
            if existing_yaml.is_empty() {
                new_block
            } else {
                format!("{}\n{}", existing_yaml.trim_end_matches('\n'), new_block)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::path::Path;
    use chrono::Utc;

    fn make_db() -> Database {
        Database::new(Path::new(":memory:")).expect("open in-memory db")
    }

    fn sample_prompt(id: &str) -> PromptConfig {
        PromptConfig {
            id: id.to_string(),
            name: format!("Prompt {}", id),
            description: Some("A test prompt".to_string()),
            target_file: "CLAUDE.md".to_string(),
            content: "You are a helpful assistant.".to_string(),
            is_active: true,
            tool_targets: Some("claude".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ── CRUD ────────────────────────────────────────────────────────────────

    #[test]
    fn save_and_list_prompt() {
        let db = make_db();
        let svc = PromptService::new(&db);
        svc.save_prompt(sample_prompt("p1")).unwrap();
        svc.save_prompt(sample_prompt("p2")).unwrap();
        let list = svc.list_prompts().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|p| p.id == "p1"));
        assert!(list.iter().any(|p| p.id == "p2"));
    }

    #[test]
    fn delete_prompt_removes_entry() {
        let db = make_db();
        let svc = PromptService::new(&db);
        svc.save_prompt(sample_prompt("p1")).unwrap();
        svc.delete_prompt("p1").unwrap();
        let list = svc.list_prompts().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_empty_returns_empty_vec() {
        let db = make_db();
        let svc = PromptService::new(&db);
        let list = svc.list_prompts().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn save_prompt_upserts_on_same_id() {
        let db = make_db();
        let svc = PromptService::new(&db);
        svc.save_prompt(sample_prompt("p1")).unwrap();
        // Overwrite with different content
        let updated = PromptConfig {
            id: "p1".to_string(),
            name: "Updated".to_string(),
            description: None,
            target_file: "AGENTS.md".to_string(),
            content: "Updated content".to_string(),
            is_active: false,
            tool_targets: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        svc.save_prompt(updated).unwrap();
        let list = svc.list_prompts().unwrap();
        assert_eq!(list.len(), 1, "upsert should not create a duplicate");
        assert_eq!(list[0].name, "Updated");
        assert_eq!(list[0].content, "Updated content");
        assert!(!list[0].is_active);
    }

    #[test]
    fn delete_nonexistent_prompt_is_ok() {
        let db = make_db();
        let svc = PromptService::new(&db);
        assert!(svc.delete_prompt("ghost").is_ok());
    }

    #[test]
    fn save_prompt_preserves_fields() {
        let db = make_db();
        let svc = PromptService::new(&db);
        let prompt = PromptConfig {
            id: "p_fields".to_string(),
            name: "Field Test".to_string(),
            description: Some("desc".to_string()),
            target_file: "CLAUDE.md".to_string(),
            content: "content here".to_string(),
            is_active: true,
            tool_targets: Some("claude,aider".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        svc.save_prompt(prompt).unwrap();
        let list = svc.list_prompts().unwrap();
        let found = list.iter().find(|p| p.id == "p_fields").unwrap();
        assert_eq!(found.description.as_deref(), Some("desc"));
        assert_eq!(found.target_file, "CLAUDE.md");
        assert_eq!(found.content, "content here");
        assert_eq!(found.tool_targets.as_deref(), Some("claude,aider"));
    }

    // ── patch_aider_system_prompt ────────────────────────────────────────────

    #[test]
    fn patch_aider_inserts_into_empty_yaml() {
        let result = PromptService::patch_aider_system_prompt("", "Be concise.");
        assert!(result.contains("system-prompt:"));
        assert!(result.contains("Be concise."));
    }

    #[test]
    fn patch_aider_appends_to_existing_yaml_without_system_prompt() {
        let existing = "model: gpt-4o\nauto-commits: false\n";
        let result = PromptService::patch_aider_system_prompt(existing, "Be concise.");
        assert!(result.contains("model: gpt-4o"));
        assert!(result.contains("system-prompt:"));
        assert!(result.contains("Be concise."));
    }

    #[test]
    fn patch_aider_replaces_existing_system_prompt() {
        let existing = "model: gpt-4o\nsystem-prompt: |\n  Old prompt text\nauto-commits: false\n";
        let result = PromptService::patch_aider_system_prompt(existing, "New prompt text");
        assert!(result.contains("New prompt text"), "new prompt should be present");
        assert!(!result.contains("Old prompt text"), "old prompt should be replaced");
        // Other keys should be preserved
        assert!(result.contains("model: gpt-4o"));
    }

    #[test]
    fn patch_aider_indents_multiline_prompt() {
        let prompt = "Line one\nLine two\nLine three";
        let result = PromptService::patch_aider_system_prompt("", prompt);
        // Each line should be indented with two spaces
        assert!(result.contains("  Line one"));
        assert!(result.contains("  Line two"));
        assert!(result.contains("  Line three"));
    }

    #[test]
    fn patch_aider_block_scalar_format() {
        let result = PromptService::patch_aider_system_prompt("", "Hello");
        // The implementation uses "system-prompt:|" (no space before the pipe)
        // followed by a newline and indented content.
        assert!(
            result.contains("system-prompt:"),
            "expected system-prompt key, got: {}",
            result
        );
        assert!(
            result.contains("|\n  Hello"),
            "expected block scalar body '|\\n  Hello', got: {}",
            result
        );
    }
}
