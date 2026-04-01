use std::path::PathBuf;
use std::fs;
use serde_json::{Value, json};
use tracing::{info, warn, error};

use crate::models::{ToolTarget, ProviderConfig, McpServer};
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

    fn home_dir() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    // ─────────────────────────────────────────────────────────
    // 公共入口：同步所有工具
    // ─────────────────────────────────────────────────────────

    pub fn sync_all(&self) {
        let providers = ProviderService::new(self.db).list_providers().unwrap_or_default();
        let mcps      = McpService::new(self.db).list_mcps().unwrap_or_default();

        self.sync_claude_code(&providers, &mcps);
        self.sync_codex(&providers);
        self.sync_gemini_cli(&providers);
        self.sync_opencode(&providers);
        self.sync_openclaw(&providers);
        self.sync_aider(&providers);
    }

    /// 仅同步单个工具（切换 Provider 时调用）
    pub fn sync_tool(&self, tool: &ToolTarget) {
        let providers = ProviderService::new(self.db).list_providers().unwrap_or_default();
        let mcps      = McpService::new(self.db).list_mcps().unwrap_or_default();

        match tool {
            ToolTarget::ClaudeCode => self.sync_claude_code(&providers, &mcps),
            ToolTarget::Codex      => self.sync_codex(&providers),
            ToolTarget::GeminiCli  => self.sync_gemini_cli(&providers),
            ToolTarget::OpenCode   => self.sync_opencode(&providers),
            ToolTarget::OpenClaw   => self.sync_openclaw(&providers),
            ToolTarget::Aider      => self.sync_aider(&providers),
        }
    }

    // ─────────────────────────────────────────────────────────
    // Claude Code  →  ~/.claude.json
    // ─────────────────────────────────────────────────────────

    fn sync_claude_code(&self, providers: &[ProviderConfig], mcps: &[McpServer]) {
        let path = Self::home_dir().join(".claude.json");

        let mut config: Value = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // 1. 写入 MCP Servers
        let mut mcp_obj = json!({});
        for mcp in mcps {
            if mcp.is_active && mcp.parsed_tool_targets().contains(&ToolTarget::ClaudeCode) {
                let args: Vec<String> = serde_json::from_str(mcp.args.as_deref().unwrap_or("[]")).unwrap_or_default();
                let env: std::collections::HashMap<String, String> =
                    serde_json::from_str(mcp.env.as_deref().unwrap_or("{}")).unwrap_or_default();
                mcp_obj[&mcp.name] = json!({
                    "command": mcp.command,
                    "args": args,
                    "env": env
                });
            }
        }
        config["mcpServers"] = mcp_obj;

        // 2. 写入激活 Provider 的环境变量
        if let Some(p) = providers.iter().find(|p| p.is_active && p.syncs_to(&ToolTarget::ClaudeCode)) {
            // 注入 ANTHROPIC_BASE_URL 和 ANTHROPIC_AUTH_TOKEN（通过 env 字段），
            // 同时保留现有的 env 字段（不覆盖用户手动设置的其他 key）
            let mut env_obj = config["env"].take();
            if env_obj.is_null() {
                env_obj = json!({});
            }
            if let Some(ref base_url) = p.base_url {
                env_obj["ANTHROPIC_BASE_URL"] = json!(base_url);
            } else {
                // 官方源：移除 base_url 覆盖
                env_obj.as_object_mut().map(|m| m.remove("ANTHROPIC_BASE_URL"));
            }
            if !p.model_name.is_empty() {
                env_obj["ANTHROPIC_MODEL"] = json!(p.model_name);
            }
            config["env"] = env_obj;
        }

        self.write_json(&path, &config, "~/.claude.json");
    }

    // ─────────────────────────────────────────────────────────
    // Codex  →  ~/.codex/config.toml  +  ~/.codex/auth.json
    // ─────────────────────────────────────────────────────────

    fn sync_codex(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers.iter().find(|p| p.is_active && p.syncs_to(&ToolTarget::Codex)) else {
            return;
        };

        let codex_dir = Self::home_dir().join(".codex");
        if let Err(e) = fs::create_dir_all(&codex_dir) {
            warn!("无法创建 ~/.codex 目录: {}", e);
            return;
        }

        // 生成 config.toml
        let base_url = p.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        // 确保以 /v1 结尾
        let base_url = if base_url.ends_with("/v1") {
            base_url.to_string()
        } else {
            format!("{}/v1", base_url.trim_end_matches('/'))
        };

        let model = if p.model_name.is_empty() { "gpt-4o".to_string() } else { p.model_name.clone() };

        let toml_content = format!(
            r#"model_provider = "newapi"
model = "{model}"
disable_response_storage = true

[model_providers.newapi]
name = "{name}"
base_url = "{base_url}"
wire_api = "responses"
requires_openai_auth = true
"#,
            model = model,
            name = p.name,
            base_url = base_url,
        );

        let config_path = codex_dir.join("config.toml");
        match fs::write(&config_path, toml_content) {
            Ok(_) => info!("已同步 ~/.codex/config.toml"),
            Err(e) => error!("写入 ~/.codex/config.toml 失败: {}", e),
        }

        // 生成 auth.json（存 API Key 引用，实际 Key 从 api_keys 表读）
        // 注意：我们不在文件里写明文 Key，而是写占位符提示 + key_id
        // 实际的 Token 注入依赖 OS Keychain（后续 Phase 实现）
        if let Some(ref key_id) = p.api_key_id {
            let auth = json!({ "_key_id": key_id, "_managed_by": "ai-singularity" });
            let auth_path = codex_dir.join("auth.json");
            self.write_json(&auth_path, &auth, "~/.codex/auth.json");
        }
    }

    // ─────────────────────────────────────────────────────────
    // Gemini CLI  →  ~/.gemini/settings.json
    // ─────────────────────────────────────────────────────────

    fn sync_gemini_cli(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers.iter().find(|p| p.is_active && p.syncs_to(&ToolTarget::GeminiCli)) else {
            return;
        };

        let gemini_dir = Self::home_dir().join(".gemini");
        if let Err(e) = fs::create_dir_all(&gemini_dir) {
            warn!("无法创建 ~/.gemini 目录: {}", e);
            return;
        }

        let settings_path = gemini_dir.join("settings.json");
        let mut settings: Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // 写入 provider 配置（保留用户其他配置项）
        if let Some(ref base_url) = p.base_url {
            settings["GOOGLE_GEMINI_BASE_URL"] = json!(base_url);
        }
        if !p.model_name.is_empty() {
            settings["model"] = json!(p.model_name);
        }

        self.write_json(&settings_path, &settings, "~/.gemini/settings.json");
    }

    // ─────────────────────────────────────────────────────────
    // OpenCode  →  ~/.config/opencode/opencode.json
    //              (Windows: %APPDATA%\opencode\opencode.json)
    // ─────────────────────────────────────────────────────────

    fn sync_opencode(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers.iter().find(|p| p.is_active && p.syncs_to(&ToolTarget::OpenCode)) else {
            return;
        };

        let config_path = Self::opencode_config_path();
        if let Some(parent) = config_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("无法创建 OpenCode 配置目录: {}", e);
                return;
            }
        }

        let mut config: Value = if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // OpenCode 使用 AI SDK 格式：{ providers: { <id>: { npm, options: { baseURL, apiKey } } } }
        let base_url = p.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        let provider_entry = json!({
            "npm": "@ai-sdk/openai-compatible",
            "options": {
                "baseURL": base_url,
                "apiKey": "{env:OPENAI_API_KEY}"  // 占位符，用户需设置环境变量
            },
            "models": {
                p.model_name.clone(): {
                    "name": p.model_name.clone()
                }
            }
        });

        if config["providers"].is_null() {
            config["providers"] = json!({});
        }
        // 用 provider id 作为 key（确保唯一）
        let safe_id = p.id.replace('-', "_");
        config["providers"][&safe_id] = provider_entry;

        self.write_json(&config_path, &config, "opencode config");
    }

    fn opencode_config_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("opencode").join("opencode.json")
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::home_dir().join(".config").join("opencode").join("opencode.json")
        }
    }

    // ─────────────────────────────────────────────────────────
    // OpenClaw  →  ~/.openclaw/config.json
    // ─────────────────────────────────────────────────────────

    fn sync_openclaw(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers.iter().find(|p| p.is_active && p.syncs_to(&ToolTarget::OpenClaw)) else {
            return;
        };

        let openclaw_dir = Self::home_dir().join(".openclaw");
        if let Err(e) = fs::create_dir_all(&openclaw_dir) {
            warn!("无法创建 ~/.openclaw 目录: {}", e);
            return;
        }

        let config_path = openclaw_dir.join("config.json");
        let mut config: Value = if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // OpenClaw 使用 OpenAI 兼容格式
        let base_url = p.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        config["openai_base_url"] = json!(base_url);
        if !p.model_name.is_empty() {
            config["model"] = json!(p.model_name);
        }

        self.write_json(&config_path, &config, "~/.openclaw/config.json");
    }

    // ─────────────────────────────────────────────────────────
    // Aider  →  ~/.aider.conf.yml
    // ─────────────────────────────────────────────────────────

    fn sync_aider(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers.iter().find(|p| p.is_active && p.syncs_to(&ToolTarget::Aider)) else {
            return;
        };

        let path = Self::home_dir().join(".aider.conf.yml");
        let mut content = String::new();

        if path.exists() {
            content = fs::read_to_string(&path).unwrap_or_default();
            // 移除旧的 model / openai-api-base 行
            content = content
                .lines()
                .filter(|l| !l.starts_with("model:") && !l.starts_with("openai-api-base:"))
                .collect::<Vec<&str>>()
                .join("\n");
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
        }

        content.push_str(&format!("model: {}\n", p.model_name));
        if let Some(ref base) = p.base_url {
            content.push_str(&format!("openai-api-base: {}\n", base));
        }

        match fs::write(&path, content) {
            Ok(_) => info!("已同步 ~/.aider.conf.yml"),
            Err(e) => error!("写入 ~/.aider.conf.yml 失败: {}", e),
        }
    }

    // ─────────────────────────────────────────────────────────
    // 工具函数
    // ─────────────────────────────────────────────────────────

    fn write_json(&self, path: &PathBuf, value: &Value, label: &str) {
        match serde_json::to_string_pretty(value) {
            Ok(content) => match fs::write(path, content) {
                Ok(_) => info!("已同步 {}", label),
                Err(e) => error!("写入 {} 失败: {}", label, e),
            },
            Err(e) => error!("序列化 {} 失败: {}", label, e),
        }
    }
}
