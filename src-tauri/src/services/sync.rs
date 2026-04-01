use std::path::{Path, PathBuf};
use std::fs;
use serde_json::{Value, json};
use tracing::{info, warn, error};

use crate::models::{AiTool, ProviderConfig, McpServer, Platform};
use crate::db::Database;
use crate::services::provider::ProviderService;
use crate::services::mcp::McpService;

pub struct SyncService<'a> {
    db: &'a Database,
}

impl<'a> SyncService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    fn get_home_dir() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    /// Update Claude Code configuration (~/.claude.json)
    pub fn sync_claude_json(&self) {
        let path = Self::get_home_dir().join(".claude.json");
        
        let mut config: Value = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // 1. Sync MCP Servers
        let mcps = McpService::new(self.db).list_mcps().unwrap_or_default();
        let mut mcp_value = json!({});
        for mcp in mcps {
            if mcp.is_active {
                // parse args
                let args_array: Vec<String> = serde_json::from_str(mcp.args.as_deref().unwrap_or("[]")).unwrap_or_default();
                // parse env
                let env_map: std::collections::HashMap<String, String> = serde_json::from_str(mcp.env.as_deref().unwrap_or("{}")).unwrap_or_default();
                
                mcp_value[&mcp.name] = json!({
                    "command": mcp.command,
                    "args": args_array,
                    "env": env_map
                });
            }
        }
        config["mcpServers"] = mcp_value;

        // 2. Sync Provider
        let providers = ProviderService::new(self.db).list_providers().unwrap_or_default();
        if let Some(p) = providers.into_iter().find(|p| p.ai_tool == AiTool::ClaudeCode && p.is_active) {
            config["primaryModel"] = json!(p.model_name);
        }

        match fs::write(&path, serde_json::to_string_pretty(&config).unwrap()) {
            Ok(_) => info!("Successfully synced ~/.claude.json"),
            Err(e) => error!("Failed to write ~/.claude.json: {}", e),
        }
    }

    /// Update Aider configuration (.aider.conf.yml usually in home or project root)
    /// We will target the home directory global config.
    pub fn sync_aider_conf(&self) {
        let providers = ProviderService::new(self.db).list_providers().unwrap_or_default();
        if let Some(p) = providers.into_iter().find(|p| p.ai_tool == AiTool::Aider && p.is_active) {
            let path = Self::get_home_dir().join(".aider.conf.yml");
                
                // For aider, we simply overwrite or append the model line. Simple text manipulation.
                let mut content = String::new();
                if path.exists() {
                    content = fs::read_to_string(&path).unwrap_or_default();
                    // Remove old model lines
                    content = content.lines().filter(|l| !l.starts_with("model:") && !l.starts_with("openai-api-base:")).collect::<Vec<&str>>().join("\n");
                    if !content.is_empty() {
                        content.push('\n');
                    }
                }
                
                content.push_str(&format!("model: {}\n", p.model_name));
                if let Some(ref base) = p.base_url {
                    // if they are using openai proxy for aider
                    content.push_str(&format!("openai-api-base: {}\n", base));
                }

                match fs::write(&path, content) {
                    Ok(_) => info!("Successfully synced ~/.aider.conf.yml"),
                    Err(e) => error!("Failed to write ~/.aider.conf.yml: {}", e),
                }
            }
    }

    /// Update VSCode Cline MCP configuration
    pub fn sync_cline_mcp(&self) {
        // usually located in AppData/Roaming/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json on Windows
        // For demonstration, we'll try the generalized path or a dummy logic if not found.
        let mut path = Self::get_home_dir();
        
        #[cfg(target_os = "windows")]
        {
            path.push("AppData");
            path.push("Roaming");
             // ... and so on
        }
        
        #[cfg(target_os = "macos")]
        {
            path.push("Library");
            path.push("Application Support");
        }
        
        path.push("Code");
        path.push("User");
        path.push("globalStorage");
        path.push("saoudrizwan.claude-dev");
        path.push("settings");
        
        // Ensure directory exists
        if !path.exists() {
            if let Err(e) = fs::create_dir_all(&path) {
                warn!("Could not create Cline settings dir: {}", e);
                return;
            }
        }
        
        path.push("cline_mcp_settings.json");

        let mut config: Value = json!({"mcpServers": {}});
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    config = val;
                }
            }
        }

        let mcps = McpService::new(self.db).list_mcps().unwrap_or_default();
        let mut mcp_value = json!({});
        for mcp in mcps {
            if mcp.is_active {
                let args_array: Vec<String> = serde_json::from_str(mcp.args.as_deref().unwrap_or("[]")).unwrap_or_default();
                let env_map: std::collections::HashMap<String, String> = serde_json::from_str(mcp.env.as_deref().unwrap_or("{}")).unwrap_or_default();
                mcp_value[&mcp.name] = json!({
                    "command": mcp.command,
                    "args": args_array,
                    "env": env_map
                });
            }
        }
        config["mcpServers"] = mcp_value;

        let _ = fs::write(&path, serde_json::to_string_pretty(&config).unwrap());
        info!("Successfully synced {}", path.display());
    }

    /// Main synchronization method: Calls all sync scripts sequentially
    pub fn sync_all(&self) {
        self.sync_claude_json();
        self.sync_aider_conf();
        self.sync_cline_mcp();
    }
}
