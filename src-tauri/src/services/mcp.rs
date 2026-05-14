use crate::db::Database;
use crate::error::AppResult;
use crate::models::McpServer;
use crate::services::sync::SyncService;
use rusqlite::params;

pub struct McpService<'a> {
    db: &'a Database,
}

impl<'a> McpService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_mcps(&self) -> AppResult<Vec<McpServer>> {
        let sql = "SELECT id, name, command, args, env, is_active, created_at, updated_at, description, tool_targets FROM mcp_servers";
        self.db
            .query_rows(sql, &[], |row| {
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
                    description: row.get(8).unwrap_or(None),
                    tool_targets: row.get(9).unwrap_or(None),
                })
            })
            .map_err(Into::into)
    }

    pub fn add_mcp(&self, mcp: McpServer) -> AppResult<()> {
        let sql = "INSERT INTO mcp_servers (id, name, command, args, env, is_active, created_at, updated_at, description, tool_targets) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)";
        self.db.execute(
            sql,
            params![
                mcp.id,
                mcp.name,
                mcp.command,
                mcp.args,
                mcp.env,
                if mcp.is_active { 1 } else { 0 },
                mcp.created_at.to_rfc3339(),
                mcp.updated_at.to_rfc3339(),
                mcp.description,
                mcp.tool_targets
            ],
        )?;
        Ok(())
    }

    pub fn toggle_mcp(&self, id: &str, is_active: bool) -> AppResult<()> {
        self.db.execute(
            "UPDATE mcp_servers SET is_active = ?1 WHERE id = ?2",
            params![if is_active { 1 } else { 0 }, id],
        )?;

        // Broadcast config sync
        SyncService::new(self.db).sync_all();

        Ok(())
    }

    pub fn delete_mcp(&self, id: &str) -> AppResult<()> {
        self.db
            .execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])?;
        SyncService::new(self.db).sync_all();
        Ok(())
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

    fn sample_mcp(id: &str) -> McpServer {
        McpServer {
            id: id.to_string(),
            name: format!("Test MCP {}", id),
            command: "npx".to_string(),
            args: Some(r#"["-y","@test/mcp"]"#.to_string()),
            env: None,
            description: Some("Test description".to_string()),
            is_active: true,
            tool_targets: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn add_and_list_mcp() {
        let db = make_db();
        let svc = McpService::new(&db);
        svc.add_mcp(sample_mcp("m1")).unwrap();
        svc.add_mcp(sample_mcp("m2")).unwrap();
        let list = svc.list_mcps().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|m| m.id == "m1"));
        assert!(list.iter().any(|m| m.id == "m2"));
    }

    #[test]
    fn toggle_mcp_active_state() {
        let db = make_db();
        let svc = McpService::new(&db);
        svc.add_mcp(sample_mcp("m1")).unwrap();
        svc.toggle_mcp("m1", false).unwrap();
        let list = svc.list_mcps().unwrap();
        assert!(!list[0].is_active);
        svc.toggle_mcp("m1", true).unwrap();
        let list = svc.list_mcps().unwrap();
        assert!(list[0].is_active);
    }

    #[test]
    fn delete_mcp_removes_entry() {
        let db = make_db();
        let svc = McpService::new(&db);
        svc.add_mcp(sample_mcp("m1")).unwrap();
        svc.delete_mcp("m1").unwrap();
        let list = svc.list_mcps().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_empty_returns_empty_vec() {
        let db = make_db();
        let svc = McpService::new(&db);
        let list = svc.list_mcps().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn add_mcp_preserves_fields() {
        let db = make_db();
        let svc = McpService::new(&db);
        let mcp = McpServer {
            id: "m_fields".to_string(),
            name: "Field Test".to_string(),
            command: "node".to_string(),
            args: Some(r#"["server.js"]"#.to_string()),
            env: Some(r#"{"PORT":"3000"}"#.to_string()),
            description: Some("desc".to_string()),
            is_active: false,
            tool_targets: Some("claude".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        svc.add_mcp(mcp).unwrap();
        let list = svc.list_mcps().unwrap();
        let found = list.iter().find(|m| m.id == "m_fields").unwrap();
        assert_eq!(found.command, "node");
        assert_eq!(found.args.as_deref(), Some(r#"["server.js"]"#));
        assert_eq!(found.env.as_deref(), Some(r#"{"PORT":"3000"}"#));
        assert_eq!(found.description.as_deref(), Some("desc"));
        assert!(!found.is_active);
        assert_eq!(found.tool_targets.as_deref(), Some("claude"));
    }

    #[test]
    fn delete_nonexistent_mcp_is_ok() {
        let db = make_db();
        let svc = McpService::new(&db);
        // Deleting a non-existent ID should not return an error
        assert!(svc.delete_mcp("ghost").is_ok());
    }
}
