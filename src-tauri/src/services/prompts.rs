use crate::error::AppResult;
use crate::models::PromptConfig;
use crate::db::Database;
use rusqlite::params;
use std::sync::Arc;
use std::fs;
use std::path::PathBuf;
use tracing::{info, error};

pub struct PromptService<'a> {
    db: &'a Database,
}

impl<'a> PromptService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_prompts(&self) -> AppResult<Vec<PromptConfig>> {
        let sql = "SELECT id, name, description, target_file, content, is_active, tool_targets, created_at, updated_at FROM prompts";
        self.db.query_rows(sql, &[], |row| {
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
        }).map_err(Into::into)
    }

    pub fn save_prompt(&self, prompt: PromptConfig) -> AppResult<()> {
        let sql = "INSERT OR REPLACE INTO prompts (id, name, description, target_file, content, is_active, tool_targets, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
        self.db.execute(sql, rusqlite::params![
            prompt.id,
            prompt.name,
            prompt.description,
            prompt.target_file,
            prompt.content,
            if prompt.is_active { 1 } else { 0 },
            prompt.tool_targets,
            prompt.created_at.to_rfc3339(),
            prompt.updated_at.to_rfc3339()
        ])?;
        Ok(())
    }

    pub fn delete_prompt(&self, id: &str) -> AppResult<()> {
        self.db.execute("DELETE FROM prompts WHERE id = ?1", params![id])?;
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
}
