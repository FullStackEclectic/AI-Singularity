use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, DirEntry};
use std::path::PathBuf;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{McpServer, PromptConfig, ProviderConfig, Platform};
use crate::services::mcp::McpService;
use crate::services::prompts::PromptService;
use crate::services::provider::ProviderService;
use crate::store::SecureStore;

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
        let keys_res: AppResult<Vec<ApiKeyExport>> = self.db.query_rows(sql, &[], |row: &rusqlite::Row| {
            let id: String = row.get(0)?;
            let platform_str: String = row.get(2)?;
            let platform = serde_json::from_str(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom);
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
        }).map_err(Into::into);
        
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
        entries.sort_by_key(|e: &DirEntry| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)));

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
            let platform_str = serde_json::to_string(&p.platform).unwrap_or_default().trim_matches('"').to_string();
            let category_str = p.category.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default().trim_matches('"').to_string());
            let tool_targets_str = p.tool_targets.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default());
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
            let platform_str = serde_json::to_string(&k.platform).unwrap_or_default().trim_matches('"').to_string();
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
            let tool_targets_str = m.tool_targets.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default());
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
