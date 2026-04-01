use crate::error::AppResult;
use crate::models::{AiTool, Platform, ProviderConfig};
use crate::db::Database;
use crate::services::sync::SyncService;
use rusqlite::params;
use std::sync::Arc;

pub struct ProviderService<'a> {
    db: &'a Database,
}

impl<'a> ProviderService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_providers(&self) -> AppResult<Vec<ProviderConfig>> {
        let sql = "SELECT id, name, ai_tool, platform, base_url, api_key_id, model_name, custom_config, is_active, created_at, updated_at FROM providers";
        self.db.query_rows(sql, &[], |row| {
            let ai_tool_str: String = row.get(2)?;
            let platform_str: String = row.get(3)?;
            let created_at_str: String = row.get(9)?;
            let updated_at_str: String = row.get(10)?;
            
            let ai_tool = serde_json::from_str(&format!("\"{}\"", ai_tool_str)).unwrap_or(AiTool::ClaudeCode);
            let platform = serde_json::from_str(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom);

            Ok(ProviderConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                ai_tool,
                platform,
                base_url: row.get(4)?,
                api_key_id: row.get(5)?,
                model_name: row.get(6)?,
                custom_config: row.get(7)?,
                is_active: row.get::<_, i32>(8)? != 0,
                created_at: created_at_str.parse().unwrap_or_default(),
                updated_at: updated_at_str.parse().unwrap_or_default(),
            })
        }).map_err(Into::into)
    }

    pub fn add_provider(&self, provider: ProviderConfig) -> AppResult<()> {
        let sql = "INSERT INTO providers (id, name, ai_tool, platform, base_url, api_key_id, model_name, custom_config, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)";
        let ai_tool_str = serde_json::to_string(&provider.ai_tool).unwrap().replace("\"", "");
        let platform_str = serde_json::to_string(&provider.platform).unwrap().replace("\"", "");

        self.db.execute(sql, params![
            provider.id,
            provider.name,
            ai_tool_str,
            platform_str,
            provider.base_url,
            provider.api_key_id,
            provider.model_name,
            provider.custom_config,
            if provider.is_active { 1 } else { 0 },
            provider.created_at.to_rfc3339(),
            provider.updated_at.to_rfc3339()
        ])?;
        Ok(())
    }

    pub fn switch_provider(&self, id: &str, ai_tool: &AiTool) -> AppResult<()> {
        let ai_tool_str = serde_json::to_string(ai_tool).unwrap().replace("\"", "");
        self.db.execute("UPDATE providers SET is_active = 0 WHERE ai_tool = ?1", params![ai_tool_str])?;
        self.db.execute("UPDATE providers SET is_active = 1 WHERE id = ?1", params![id])?;
        
        // Broadcast config sync
        SyncService::new(self.db).sync_all();
        
        Ok(())
    }

    pub fn delete_provider(&self, id: &str) -> AppResult<()> {
        self.db.execute("DELETE FROM providers WHERE id = ?1", params![id])?;
        SyncService::new(self.db).sync_all();
        Ok(())
    }
}
