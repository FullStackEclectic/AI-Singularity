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
