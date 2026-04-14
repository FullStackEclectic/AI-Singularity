use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::db::Database;
use crate::models::{McpServer, ProviderConfig, ToolTarget};
use crate::services::backup::BackupService;
use crate::services::mcp::McpService;
use crate::services::provider::ProviderService;

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

    pub fn sync_codex_dir_with_provider_id(
        &self,
        codex_dir: &PathBuf,
        provider_id: Option<&str>,
    ) -> Result<(), String> {
        let providers = ProviderService::new(self.db)
            .list_providers()
            .map_err(|e| e.to_string())?;
        let mcps = McpService::new(self.db)
            .list_mcps()
            .map_err(|e| e.to_string())?;

        let provider = if let Some(provider_id) = provider_id.filter(|item| !item.trim().is_empty()) {
            providers
                .iter()
                .find(|item| item.id == provider_id && item.syncs_to(&ToolTarget::Codex))
        } else {
            providers
                .iter()
                .find(|item| item.is_active && item.syncs_to(&ToolTarget::Codex))
        }
        .ok_or_else(|| "未找到可用于 Codex 的 Provider".to_string())?;

        self.write_codex_config(codex_dir, provider, &mcps)
    }

    // ─────────────────────────────────────────────────────────
    // 公共入口：同步所有工具
    // ─────────────────────────────────────────────────────────

    pub fn sync_all(&self) {
        // === 执行写操作前自动备份 ===
        let path = std::path::Path::new(&self.db.path);
        if let Some(parent) = path.parent() {
            let app_data_dir = parent.to_path_buf();
            let backup_service = BackupService::new(self.db, app_data_dir);
            if let Err(e) = backup_service.create_auto_backup() {
                warn!("SyncService: 自动备份失败: {}", e);
            }
        }

        let providers = ProviderService::new(self.db)
            .list_providers()
            .unwrap_or_default();
        let mcps = McpService::new(self.db).list_mcps().unwrap_or_default();
        let prompts = crate::services::prompts::PromptService::new(self.db)
            .list_prompts()
            .unwrap_or_default();

        self.sync_claude_code(&providers, &mcps, &prompts);
        self.sync_codex(&providers, &mcps);
        self.sync_gemini_cli(&providers);
        self.sync_opencode(&providers, &mcps);
        self.sync_openclaw(&providers);
        self.sync_aider(&providers, &prompts);
    }

    /// 仅同步单个工具（切换 Provider 时调用）
    #[allow(dead_code)]
    pub fn sync_tool(&self, tool: &ToolTarget) {
        // === 执行写操作前自动备份 ===
        let path = std::path::Path::new(&self.db.path);
        if let Some(parent) = path.parent() {
            let app_data_dir = parent.to_path_buf();
            let backup_service = BackupService::new(self.db, app_data_dir);
            if let Err(e) = backup_service.create_auto_backup() {
                warn!("SyncService: 自动备份失败: {}", e);
            }
        }

        let providers = ProviderService::new(self.db)
            .list_providers()
            .unwrap_or_default();
        let mcps = McpService::new(self.db).list_mcps().unwrap_or_default();
        let prompts = crate::services::prompts::PromptService::new(self.db)
            .list_prompts()
            .unwrap_or_default();

        match tool {
            ToolTarget::ClaudeCode => self.sync_claude_code(&providers, &mcps, &prompts),
            ToolTarget::Codex => self.sync_codex(&providers, &mcps),
            ToolTarget::GeminiCli => self.sync_gemini_cli(&providers),
            ToolTarget::OpenCode => self.sync_opencode(&providers, &mcps),
            ToolTarget::OpenClaw => self.sync_openclaw(&providers),
            ToolTarget::Aider => self.sync_aider(&providers, &prompts),
        }
    }

    // ─────────────────────────────────────────────────────────
    // Claude Code  →  ~/.claude.json
    // ─────────────────────────────────────────────────────────

    fn sync_claude_code(
        &self,
        providers: &[ProviderConfig],
        mcps: &[McpServer],
        prompts: &[crate::models::PromptConfig],
    ) {
        let claude_dir = Self::home_dir().join(".claude");
        let _ = fs::create_dir_all(&claude_dir);
        let path = claude_dir.join("settings.json");

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
                let args: Vec<String> =
                    serde_json::from_str(mcp.args.as_deref().unwrap_or("[]")).unwrap_or_default();
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
        if let Some(p) = providers
            .iter()
            .find(|p| p.is_active && p.syncs_to(&ToolTarget::ClaudeCode))
        {
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
                env_obj
                    .as_object_mut()
                    .map(|m| m.remove("ANTHROPIC_BASE_URL"));
            }
            if !p.model_name.is_empty() {
                env_obj["ANTHROPIC_MODEL"] = json!(p.model_name);
            }
            config["env"] = env_obj;
        }

        // 3. 注入 Prompt Templates (customInstructions)
        let claude_prompts: Vec<String> = prompts
            .iter()
            .filter(|p| p.is_active && p.parsed_tool_targets().contains(&ToolTarget::ClaudeCode))
            .map(|p| p.content.clone())
            .collect();

        if !claude_prompts.is_empty() {
            let combined_prompt = claude_prompts.join("\n\n---\n\n");
            config["customInstructions"] = json!(combined_prompt);
        } else {
            // 如果 AI Singularity 中没有启用 Claude Code 相关的 Prompt，清除该键（交回给官方默认）
            config
                .as_object_mut()
                .map(|m| m.remove("customInstructions"));
        }

        self.write_json(&path, &config, "~/.claude/settings.json");
    }

    // ─────────────────────────────────────────────────────────
    // Codex  →  ~/.codex/config.toml  +  ~/.codex/auth.json
    // ─────────────────────────────────────────────────────────

    fn sync_codex(&self, providers: &[ProviderConfig], mcps: &[McpServer]) {
        let Some(p) = providers
            .iter()
            .find(|p| p.is_active && p.syncs_to(&ToolTarget::Codex))
        else {
            return;
        };

        let codex_dir = Self::home_dir().join(".codex");
        if let Err(e) = fs::create_dir_all(&codex_dir) {
            warn!("无法创建 ~/.codex 目录: {}", e);
            return;
        }

        if let Err(e) = self.write_codex_config(&codex_dir, p, mcps) {
            error!("写入 ~/.codex/config.toml 失败: {}", e);
            return;
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

    fn write_codex_config(
        &self,
        codex_dir: &PathBuf,
        p: &ProviderConfig,
        mcps: &[McpServer],
    ) -> Result<(), String> {
        // 生成 config.toml
        let base_url = p.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        // 确保以 /v1 结尾
        let base_url = if base_url.ends_with("/v1") {
            base_url.to_string()
        } else {
            format!("{}/v1", base_url.trim_end_matches('/'))
        };

        let model = if p.model_name.is_empty() {
            "gpt-4o".to_string()
        } else {
            p.model_name.clone()
        };

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

        let mut toml_doc = toml_content
            .parse::<toml_edit::DocumentMut>()
            .unwrap_or_default();

        // 渲染 MCP 配置
        let mut mcp_servers_tbl = toml_edit::Table::new();
        let mut added_mcps = false;

        for mcp in mcps {
            if mcp.is_active && mcp.parsed_tool_targets().contains(&ToolTarget::Codex) {
                let mut server_tbl = toml_edit::Table::new();

                // 类型如果是 stdio / sse 根据 command 等判断, 这里简单以 command 是否为空区分？
                // 如果有 command 则是 stdio, 如果是 url 则是 http
                let cmd = mcp.command.clone();
                let typ = if cmd.starts_with("http") {
                    "http"
                } else {
                    "stdio"
                };

                server_tbl["type"] = toml_edit::value(typ);

                if typ == "stdio" {
                    server_tbl["command"] = toml_edit::value(cmd);

                    if let Some(args) = &mcp.args {
                        if let Ok(args_arr) = serde_json::from_str::<Vec<String>>(args) {
                            let mut arr = toml_edit::Array::default();
                            for a in args_arr {
                                arr.push(a);
                            }
                            if !arr.is_empty() {
                                server_tbl["args"] =
                                    toml_edit::Item::Value(toml_edit::Value::Array(arr));
                            }
                        }
                    }
                    if let Some(env) = &mcp.env {
                        if let Ok(env_map) =
                            serde_json::from_str::<std::collections::HashMap<String, String>>(env)
                        {
                            let mut env_tbl = toml_edit::Table::new();
                            for (k, v) in env_map {
                                env_tbl[&k] = toml_edit::value(v);
                            }
                            if !env_tbl.is_empty() {
                                server_tbl["env"] = toml_edit::Item::Table(env_tbl);
                            }
                        }
                    }
                } else if typ == "http" {
                    server_tbl["url"] = toml_edit::value(cmd);
                }

                mcp_servers_tbl[&mcp.name] = toml_edit::Item::Table(server_tbl);
                added_mcps = true;
            }
        }

        if added_mcps {
            toml_doc["mcp_servers"] = toml_edit::Item::Table(mcp_servers_tbl);
        }

        fs::write(&config_path, toml_doc.to_string())
            .map_err(|e| format!("写入 {} 失败: {}", config_path.display(), e))?;
        info!("已同步 {}", config_path.display());
        Ok(())
    }

    // ─────────────────────────────────────────────────────────
    // Gemini CLI  →  ~/.gemini/settings.json
    // ─────────────────────────────────────────────────────────

    fn sync_gemini_cli(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers
            .iter()
            .find(|p| p.is_active && p.syncs_to(&ToolTarget::GeminiCli))
        else {
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

    fn sync_opencode(&self, providers: &[ProviderConfig], mcps: &[McpServer]) {
        let Some(p) = providers
            .iter()
            .find(|p| p.is_active && p.syncs_to(&ToolTarget::OpenCode))
        else {
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

        // 注入 MCP
        let mut mcp_obj = json!({});
        for mcp in mcps {
            if mcp.is_active && mcp.parsed_tool_targets().contains(&ToolTarget::OpenCode) {
                let args: Vec<String> =
                    serde_json::from_str(mcp.args.as_deref().unwrap_or("[]")).unwrap_or_default();
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

        self.write_json(&config_path, &config, "opencode config");
    }

    fn opencode_config_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata)
                .join("opencode")
                .join("opencode.json")
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::home_dir()
                .join(".config")
                .join("opencode")
                .join("opencode.json")
        }
    }

    // ─────────────────────────────────────────────────────────
    // OpenClaw  →  ~/.openclaw/config.json
    // ─────────────────────────────────────────────────────────

    fn sync_openclaw(&self, providers: &[ProviderConfig]) {
        let Some(p) = providers
            .iter()
            .find(|p| p.is_active && p.syncs_to(&ToolTarget::OpenClaw))
        else {
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

    fn sync_aider(&self, providers: &[ProviderConfig], prompts: &[crate::models::PromptConfig]) {
        let Some(p) = providers
            .iter()
            .find(|p| p.is_active && p.syncs_to(&ToolTarget::Aider))
        else {
            return;
        };

        let path = Self::home_dir().join(".aider.conf.yml");
        let mut content = String::new();

        if path.exists() {
            content = fs::read_to_string(&path).unwrap_or_default();
            // 移除旧的 model / openai-api-base / architect-system-prompt 行
            content = content
                .lines()
                .filter(|l| {
                    !l.starts_with("model:")
                        && !l.starts_with("openai-api-base:")
                        && !l.starts_with("architect-system-prompt:")
                })
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

        // 注入 Prompt Template (作为 architect_system_prompt 或 read)
        let aider_prompts: Vec<String> = prompts
            .iter()
            .filter(|p| p.is_active && p.parsed_tool_targets().contains(&ToolTarget::Aider))
            .map(|p| p.content.clone())
            .collect();

        if !aider_prompts.is_empty() {
            let combined = aider_prompts.join("\n\n---\n\n");
            // 将换行进行缩进处理，以便写入 YAML
            let indented: String = combined
                .lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            content.push_str(&format!("architect-system-prompt: |-\n{}\n", indented));
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
