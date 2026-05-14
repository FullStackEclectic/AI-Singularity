use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{McpServer, Platform, PromptConfig, ProviderConfig};
use crate::services::mcp::McpService;
use crate::services::prompts::PromptService;
use crate::services::provider::ProviderService;
use crate::store::SecureStore;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, DirEntry};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    pub version: u32,
    pub timestamp: String,
    pub providers: Vec<ProviderConfig>,
    pub api_keys: Vec<ApiKeyExport>,
    pub mcp_servers: Vec<McpServer>,
    pub prompts: Vec<PromptConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyExport {
    pub id: String,
    pub name: String,
    pub platform: crate::models::Platform,
    pub base_url: Option<String>,
    pub key_preview: String,
    pub secret: Option<String>, // 导出时附带，明文存储在备份中！
    pub notes: Option<String>,
}

pub struct BackupService<'a> {
    db: &'a Database,
    app_data_dir: PathBuf,
}

impl<'a> BackupService<'a> {
    pub fn new(db: &'a Database, app_data_dir: PathBuf) -> Self {
        Self { db, app_data_dir }
    }

    /// 导出全量配置（包括并解密所有 API Key）
    pub fn export_config(&self) -> AppResult<BackupData> {
        let providers = ProviderService::new(self.db).list_providers()?;
        let mcp_servers = McpService::new(self.db).list_mcps()?;
        let prompts = PromptService::new(self.db).list_prompts()?;

        // 读取 Keys 并解密
        let mut api_keys_export = Vec::new();
        let sql = "SELECT id, name, platform, base_url, key_preview, notes FROM api_keys";
        let keys_res: AppResult<Vec<ApiKeyExport>> = self
            .db
            .query_rows(sql, &[], |row: &rusqlite::Row| {
                let id: String = row.get(0)?;
                let platform_str: String = row.get(2)?;
                let platform = serde_json::from_str(&format!("\"{}\"", platform_str))
                    .unwrap_or(Platform::Custom);
                let secret = match SecureStore::get_key(&id) {
                    Ok(s) => Some(s),
                    Err(_) => None,
                };
                Ok(ApiKeyExport {
                    id,
                    name: row.get(1)?,
                    platform,
                    base_url: row.get(3).unwrap_or(None),
                    key_preview: row.get(4)?,
                    secret,
                    notes: row.get(5).unwrap_or(None),
                })
            })
            .map_err(Into::into);

        if let Ok(keys) = keys_res {
            api_keys_export = keys;
        }

        Ok(BackupData {
            version: 1,
            timestamp: Utc::now().to_rfc3339(),
            providers,
            api_keys: api_keys_export,
            mcp_servers,
            prompts,
        })
    }

    /// 执行全量备份逻辑并写入文件
    pub fn create_auto_backup(&self) -> AppResult<()> {
        let data = self.export_config()?;
        let json_str = serde_json::to_string_pretty(&data)?;

        let backup_dir = self.app_data_dir.join("backups");
        fs::create_dir_all(&backup_dir)?;

        let filename = format!("backup-{}.json", Utc::now().format("%Y%m%d-%H%M%S"));
        let file_path = backup_dir.join(filename);

        fs::write(&file_path, json_str)?;

        // 轮换：最多保留 10 份
        self.rotate_backups(&backup_dir, 10)?;

        Ok(())
    }

    /// 轮换旧备份
    fn rotate_backups(&self, backup_dir: &PathBuf, max_files: usize) -> Result<(), AppError> {
        let mut entries: Vec<DirEntry> = fs::read_dir(backup_dir)?
            .filter_map(Result::ok)
            .filter(|e: &DirEntry| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();

        // 按最后修改时间排序（从新到旧）
        entries.sort_by_key(|e: &DirEntry| {
            std::cmp::Reverse(
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            )
        });

        if entries.len() > max_files {
            for v in entries.iter().skip(max_files) {
                let _ = fs::remove_file(v.path());
            }
        }
        Ok(())
    }

    /// 从 JSON 导入配置（支持增量/覆盖）
    pub fn import_config(&self, json_data: &str) -> AppResult<()> {
        let data: BackupData = serde_json::from_str(json_data)?;

        // Providers
        for p in data.providers {
            let platform_str = serde_json::to_string(&p.platform)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            let category_str = p.category.as_ref().map(|c| {
                serde_json::to_string(c)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string()
            });
            let tool_targets_str = p
                .tool_targets
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap_or_default());
            self.db.execute(
                "REPLACE INTO providers (id, name, platform, category, base_url, model_name, is_active, tool_targets, icon, icon_color, website_url, api_key_url, notes, extra_config, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    &p.id, 
                    &p.name, 
                    &platform_str, 
                    category_str.as_deref(), 
                    p.base_url.as_deref(), 
                    &p.model_name, 
                    &p.is_active, 
                    tool_targets_str.as_deref(), 
                    p.icon.as_deref(), 
                    p.icon_color.as_deref(), 
                    p.website_url.as_deref(), 
                    p.api_key_url.as_deref(), 
                    p.notes.as_deref(), 
                    p.extra_config.as_deref(), 
                    &p.created_at.to_rfc3339(), 
                    &p.updated_at.to_rfc3339()
                ],
            )?;
        }

        // ApiKeys
        for k in data.api_keys {
            let platform_str = serde_json::to_string(&k.platform)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            self.db.execute(
                "REPLACE INTO api_keys (id, name, platform, base_url, key_preview, status, notes, created_at, last_checked_at, key_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'unknown', ?6, ?7, NULL, '')",
                rusqlite::params![&k.id, &k.name, &platform_str, k.base_url.as_deref(), &k.key_preview, k.notes.as_deref(), Utc::now().to_rfc3339()],
            )?;
            if let Some(sec) = k.secret {
                let _ = SecureStore::store_key(&k.id, &sec);
            }
        }

        // McpServers
        for m in data.mcp_servers {
            let tool_targets_str = m
                .tool_targets
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap_or_default());
            self.db.execute(
                "REPLACE INTO mcp_servers (id, name, command, args, env, description, is_active, tool_targets, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    &m.id, 
                    &m.name, 
                    &m.command, 
                    m.args.as_deref(), 
                    m.env.as_deref(), 
                    m.description.as_deref(), 
                    &m.is_active, 
                    tool_targets_str.as_deref(), 
                    &m.created_at.to_rfc3339(), 
                    &m.updated_at.to_rfc3339()
                ],
            )?;
        }

        // Prompts
        for pr in data.prompts {
            self.db.execute(
                "REPLACE INTO prompts (id, name, target_file, content, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    &pr.id, 
                    &pr.name, 
                    &pr.target_file, 
                    &pr.content, 
                    &pr.is_active, 
                    &pr.created_at.to_rfc3339(), 
                    &pr.updated_at.to_rfc3339()
                ],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{McpServer, Platform, PromptConfig, ProviderConfig};
    use crate::services::mcp::McpService;
    use crate::services::prompts::PromptService;
    use crate::services::provider::ProviderService;
    use chrono::Utc;
    use std::fs;
    use std::path::Path;

    fn make_db() -> Database {
        Database::new(Path::new(":memory:")).expect("open in-memory db")
    }

    fn test_backup_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ais_test_backup_{}", suffix));
        fs::create_dir_all(&dir).expect("create test backup dir");
        dir
    }

    fn cleanup_dir(dir: &PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    fn sample_provider(id: &str) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            name: format!("Provider {}", id),
            platform: Platform::OpenAI,
            category: None,
            base_url: None,
            api_key_id: None,
            model_name: "gpt-4o".to_string(),
            is_active: false,
            tool_targets: Some(r#"["claude_code"]"#.to_string()),
            icon: None,
            icon_color: None,
            website_url: None,
            api_key_url: None,
            notes: None,
            extra_config: None,
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_mcp(id: &str) -> McpServer {
        McpServer {
            id: id.to_string(),
            name: format!("MCP {}", id),
            command: "npx".to_string(),
            args: None,
            env: None,
            description: None,
            is_active: true,
            tool_targets: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_prompt(id: &str) -> PromptConfig {
        PromptConfig {
            id: id.to_string(),
            name: format!("Prompt {}", id),
            description: None,
            target_file: "CLAUDE.md".to_string(),
            content: "Be helpful.".to_string(),
            is_active: true,
            tool_targets: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn export_config_returns_empty_when_no_data() {
        let db = make_db();
        let dir = test_backup_dir("empty");
        let svc = BackupService::new(&db, dir.clone());

        let data = svc.export_config().unwrap();
        assert!(data.providers.is_empty(), "providers should be empty");
        assert!(data.mcp_servers.is_empty(), "mcp_servers should be empty");
        assert!(data.prompts.is_empty(), "prompts should be empty");

        cleanup_dir(&dir);
    }

    #[test]
    fn import_config_restores_providers() {
        let db = make_db();
        let dir = test_backup_dir("providers");
        let svc = BackupService::new(&db, dir.clone());

        // Add a provider, export, then clear and re-import
        ProviderService::new(&db).add_provider(sample_provider("p1")).unwrap();
        let exported = svc.export_config().unwrap();
        let json = serde_json::to_string(&exported).unwrap();

        // Clear the table
        db.execute("DELETE FROM providers", &[]).unwrap();
        assert!(ProviderService::new(&db).list_providers().unwrap().is_empty());

        // Import
        svc.import_config(&json).unwrap();
        let restored = ProviderService::new(&db).list_providers().unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, "p1");

        cleanup_dir(&dir);
    }

    #[test]
    fn import_config_restores_mcp_servers() {
        let db = make_db();
        let dir = test_backup_dir("mcps");
        let svc = BackupService::new(&db, dir.clone());

        McpService::new(&db).add_mcp(sample_mcp("m1")).unwrap();
        let exported = svc.export_config().unwrap();
        let json = serde_json::to_string(&exported).unwrap();

        db.execute("DELETE FROM mcp_servers", &[]).unwrap();
        assert!(McpService::new(&db).list_mcps().unwrap().is_empty());

        svc.import_config(&json).unwrap();
        let restored = McpService::new(&db).list_mcps().unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, "m1");

        cleanup_dir(&dir);
    }

    #[test]
    fn import_config_restores_prompts() {
        let db = make_db();
        let dir = test_backup_dir("prompts");
        let svc = BackupService::new(&db, dir.clone());

        PromptService::new(&db).save_prompt(sample_prompt("pr1")).unwrap();
        let exported = svc.export_config().unwrap();
        let json = serde_json::to_string(&exported).unwrap();

        db.execute("DELETE FROM prompts", &[]).unwrap();
        assert!(PromptService::new(&db).list_prompts().unwrap().is_empty());

        svc.import_config(&json).unwrap();
        let restored = PromptService::new(&db).list_prompts().unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, "pr1");

        cleanup_dir(&dir);
    }

    #[test]
    fn rotate_backups_keeps_max_files() {
        let db = make_db();
        let dir = test_backup_dir("rotate");
        let backup_dir = dir.join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create 12 dummy backup files with slightly different modification times
        for i in 0..12 {
            let path = backup_dir.join(format!("backup-2026010{:02}-{:06}.json", i / 10, i));
            fs::write(&path, format!("{{\"version\":{}}}", i)).unwrap();
            // Small sleep to ensure distinct mtime ordering on fast filesystems
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let entries_before: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(entries_before.len(), 12, "should have 12 files before rotation");

        // create_auto_backup exports (empty db is fine) and then rotates to 10
        let svc = BackupService::new(&db, dir.clone());
        svc.create_auto_backup().unwrap();

        let entries_after: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(entries_after.len(), 10, "rotation should keep exactly 10 files");

        cleanup_dir(&dir);
    }
}
