use crate::error::AppResult;
use crate::models::McpServer;
use crate::db::Database;
use crate::services::sync::SyncService;
use rusqlite::params;
use std::sync::Arc;

pub struct McpService<'a> {
    db: &'a Database,
}

impl<'a> McpService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_mcps(&self) -> AppResult<Vec<McpServer>> {
        let sql = "SELECT id, name, command, args, env, is_active, created_at, updated_at FROM mcp_servers";
        self.db.query_rows(sql, &[], |row| {
            let created_at_str: String = row.get(6)?;
            let updated_at_str: String = row.get(7)?;
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                command: row.get(2)?,
                args: row.get(3)?,
                env: row.get(4)?,
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: created_at_str.parse().unwrap_or_default(),
                updated_at: updated_at_str.parse().unwrap_or_default(),
            })
        }).map_err(Into::into)
    }

    pub fn add_mcp(&self, mcp: McpServer) -> AppResult<()> {
        let sql = "INSERT INTO mcp_servers (id, name, command, args, env, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
        self.db.execute(sql, params![
            mcp.id,
            mcp.name,
            mcp.command,
            mcp.args,
            mcp.env,
            if mcp.is_active { 1 } else { 0 },
            mcp.created_at.to_rfc3339(),
            mcp.updated_at.to_rfc3339()
        ])?;
        Ok(())
    }

    pub fn toggle_mcp(&self, id: &str, is_active: bool) -> AppResult<()> {
        self.db.execute("UPDATE mcp_servers SET is_active = ?1 WHERE id = ?2", params![if is_active { 1 } else { 0 }, id])?;
        
        // Broadcast config sync
        SyncService::new(self.db).sync_all();

        Ok(())
    }

    pub fn delete_mcp(&self, id: &str) -> AppResult<()> {
        self.db.execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])?;
        SyncService::new(self.db).sync_all();
        Ok(())
    }
}
