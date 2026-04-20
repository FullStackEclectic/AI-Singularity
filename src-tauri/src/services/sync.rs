use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, DocumentMut, Item, Table, Value as TomlValue};
use tracing::{error, info, warn};

use crate::db::Database;
use crate::models::{McpServer, ProviderConfig, ToolTarget};
use crate::services::backup::BackupService;
use crate::services::mcp::McpService;
use crate::services::provider::ProviderService;
use crate::store::SecureStore;

const CODEX_MANAGED_PROVIDER_KEY: &str = "ai_singularity";
const CODEX_LEGACY_PROVIDER_KEY: &str = "newapi";

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

        let provider =
            if let Some(provider_id) = provider_id.filter(|item| !item.trim().is_empty()) {
                providers
                    .iter()
                    .find(|item| item.id == provider_id && item.syncs_to(&ToolTarget::Codex))
            } else {
                providers
                    .iter()
                    .find(|item| item.is_active && item.syncs_to(&ToolTarget::Codex))
            }
            .ok_or_else(|| "未找到可用于 Codex 的 Provider".to_string())?;

        self.write_codex_config(codex_dir, provider, &mcps)?;
        self.write_codex_auth(codex_dir, provider)
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
            let existing_env = config["env"].take();
            let api_key_secret = self.read_provider_api_key_secret(p);
            config["env"] = build_claude_env(existing_env, p, api_key_secret.as_deref());
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

        if let Err(e) = self.write_codex_auth(&codex_dir, p) {
            warn!("写入 ~/.codex/auth.json 失败: {}", e);
        }
    }

    fn write_codex_config(
        &self,
        codex_dir: &PathBuf,
        p: &ProviderConfig,
        mcps: &[McpServer],
    ) -> Result<(), String> {
        let config_path = codex_dir.join("config.toml");
        let existing_content = if config_path.exists() {
            Some(
                fs::read_to_string(&config_path)
                    .map_err(|e| format!("读取 {} 失败: {}", config_path.display(), e))?,
            )
        } else {
            None
        };
        let rendered = render_codex_config(existing_content.as_deref(), p, mcps)?;

        fs::write(&config_path, rendered)
            .map_err(|e| format!("写入 {} 失败: {}", config_path.display(), e))?;
        info!("已同步 {}", config_path.display());
        Ok(())
    }

    fn write_codex_auth(&self, codex_dir: &PathBuf, p: &ProviderConfig) -> Result<(), String> {
        let Some(key_id) = p
            .api_key_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(());
        };

        let secret = SecureStore::get_key(key_id)
            .map_err(|e| format!("读取 Keychain 中的 Codex API Key 失败: {}", e))?;
        let auth = build_codex_auth_json(&secret);
        let auth_path = codex_dir.join("auth.json");
        self.write_json(&auth_path, &auth, "~/.codex/auth.json");
        Ok(())
    }

    fn read_provider_api_key_secret(&self, p: &ProviderConfig) -> Option<String> {
        let key_id = p.api_key_id.as_deref()?.trim();
        if key_id.is_empty() {
            return None;
        }
        match SecureStore::get_key(key_id) {
            Ok(secret) => Some(secret),
            Err(e) => {
                warn!("读取 Provider API Key 失败 (provider={}): {}", p.id, e);
                None
            }
        }
    }

    // ─────────────────────────────────────────────────────────
    // Gemini CLI  →  ~/.gemini/settings.json + ~/.gemini/.env
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
        let settings: Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };
        let extra_config = parse_provider_extra_config(p.extra_config.as_deref());
        let api_key_secret = self.read_provider_api_key_secret(p);

        let settings = build_gemini_settings(settings, p, &extra_config);

        self.write_json(&settings_path, &settings, "~/.gemini/settings.json");

        let env_path = gemini_dir.join(".env");
        let existing_env = if env_path.exists() {
            fs::read_to_string(&env_path).ok()
        } else {
            None
        };
        let env_content = render_gemini_env_file(
            existing_env.as_deref(),
            p,
            &extra_config,
            api_key_secret.as_deref(),
        );
        match fs::write(&env_path, env_content) {
            Ok(_) => info!("已同步 ~/.gemini/.env"),
            Err(e) => error!("写入 ~/.gemini/.env 失败: {}", e),
        }
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
        let extra_config = parse_provider_extra_config(p.extra_config.as_deref());
        let base_url = p.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        let model = resolve_tool_model(&extra_config, "open_code", Some(p.model_name.as_str()))
            .unwrap_or_else(|| p.model_name.clone());
        let api_key_value = self
            .read_provider_api_key_secret(p)
            .unwrap_or_else(|| "{env:OPENAI_API_KEY}".to_string());
        let provider_entry = json!({
            "npm": "@ai-sdk/openai-compatible",
            "options": {
                "baseURL": base_url,
                "apiKey": api_key_value
            },
            "models": {
                model.clone(): {
                    "name": model.clone()
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
        let extra_config = parse_provider_extra_config(p.extra_config.as_deref());
        let base_url = p.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        config["openai_base_url"] = json!(base_url);
        set_json_string_field(
            &mut config,
            "model",
            resolve_tool_model(&extra_config, "open_claw", Some(p.model_name.as_str())),
        );

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

        let extra_config = parse_provider_extra_config(p.extra_config.as_deref());
        let aider_model = resolve_tool_model(&extra_config, "aider", Some(p.model_name.as_str()))
            .unwrap_or_else(|| p.model_name.clone());
        content.push_str(&format!("model: {}\n", aider_model));
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

fn build_codex_auth_json(secret: &str) -> Value {
    json!({
        "OPENAI_API_KEY": secret.trim(),
    })
}

fn parse_provider_extra_config(raw: Option<&str>) -> Value {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| json!({}))
}

fn get_extra_string(extra_config: &Value, key: &str) -> Option<String> {
    extra_config
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn get_tool_config_string(extra_config: &Value, tool_key: &str, field: &str) -> Option<String> {
    extra_config
        .get("tool_configs")
        .and_then(|value| value.get(tool_key))
        .and_then(|value| value.get(field))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn resolve_tool_model(
    extra_config: &Value,
    tool_key: &str,
    default_model: Option<&str>,
) -> Option<String> {
    get_tool_config_string(extra_config, tool_key, "model").or_else(|| {
        default_model
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    })
}

fn set_json_string_field(root: &mut Value, key: &str, value: Option<String>) {
    if !root.is_object() {
        *root = json!({});
    }
    if let Some(obj) = root.as_object_mut() {
        if let Some(value) = value {
            obj.insert(key.to_string(), Value::String(value));
        } else {
            obj.remove(key);
        }
    }
}

fn set_json_nested_string_field(root: &mut Value, path: &[&str], value: Option<String>) {
    if path.is_empty() {
        return;
    }
    if !root.is_object() {
        *root = json!({});
    }

    let mut current = root;
    for segment in &path[..path.len() - 1] {
        if current
            .get(segment)
            .and_then(|child| child.as_object())
            .is_none()
        {
            current[*segment] = json!({});
        }
        current = &mut current[*segment];
    }

    set_json_string_field(current, path[path.len() - 1], value);
}

fn build_claude_env(
    existing_env: Value,
    provider: &ProviderConfig,
    api_key_secret: Option<&str>,
) -> Value {
    let extra_config = parse_provider_extra_config(provider.extra_config.as_deref());
    let mut env_obj = if existing_env.is_object() {
        existing_env
    } else {
        json!({})
    };

    let auth_field = get_extra_string(&extra_config, "apiKeyField")
        .filter(|value| {
            value.eq_ignore_ascii_case("ANTHROPIC_API_KEY")
                || value.eq_ignore_ascii_case("ANTHROPIC_AUTH_TOKEN")
        })
        .unwrap_or_else(|| "ANTHROPIC_API_KEY".to_string());
    set_json_string_field(
        &mut env_obj,
        "ANTHROPIC_BASE_URL",
        provider
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
    );
    set_json_string_field(
        &mut env_obj,
        "ANTHROPIC_MODEL",
        resolve_tool_model(
            &extra_config,
            "claude_code",
            Some(provider.model_name.as_str()),
        ),
    );
    set_json_string_field(
        &mut env_obj,
        "ANTHROPIC_REASONING_MODEL",
        get_tool_config_string(&extra_config, "claude_code", "reasoningModel"),
    );
    set_json_string_field(
        &mut env_obj,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        get_tool_config_string(&extra_config, "claude_code", "haikuModel"),
    );
    set_json_string_field(
        &mut env_obj,
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        get_tool_config_string(&extra_config, "claude_code", "sonnetModel"),
    );
    set_json_string_field(
        &mut env_obj,
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        get_tool_config_string(&extra_config, "claude_code", "opusModel"),
    );

    if !auth_field.eq_ignore_ascii_case("ANTHROPIC_API_KEY") {
        set_json_string_field(&mut env_obj, "ANTHROPIC_API_KEY", None);
    }
    if !auth_field.eq_ignore_ascii_case("ANTHROPIC_AUTH_TOKEN") {
        set_json_string_field(&mut env_obj, "ANTHROPIC_AUTH_TOKEN", None);
    }
    set_json_string_field(
        &mut env_obj,
        &auth_field,
        api_key_secret
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
    );

    env_obj
}

fn build_gemini_settings(
    existing_settings: Value,
    provider: &ProviderConfig,
    extra_config: &Value,
) -> Value {
    let mut settings = if existing_settings.is_object() {
        existing_settings
    } else {
        json!({})
    };

    set_json_string_field(
        &mut settings,
        "GOOGLE_GEMINI_BASE_URL",
        provider
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
    );
    set_json_string_field(
        &mut settings,
        "model",
        resolve_tool_model(
            extra_config,
            "gemini_cli",
            Some(provider.model_name.as_str()),
        ),
    );
    set_json_string_field(
        &mut settings,
        "projectId",
        get_extra_string(extra_config, "projectId"),
    );
    set_json_nested_string_field(
        &mut settings,
        &["security", "auth", "selectedType"],
        Some("gemini-api-key".to_string()),
    );

    settings
}

fn gemini_managed_env_keys() -> &'static [&'static str] {
    &[
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "GOOGLE_GEMINI_BASE_URL",
        "GOOGLE_CLOUD_PROJECT",
        "GEMINI_MODEL",
    ]
}

fn gemini_env_injection_mode(extra_config: &Value) -> &str {
    match get_extra_string(extra_config, "envInjection").as_deref() {
        Some("legacy") => "legacy",
        _ => "standard",
    }
}

fn build_gemini_env_entries(
    provider: &ProviderConfig,
    extra_config: &Value,
    api_key_secret: Option<&str>,
) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let env_mode = gemini_env_injection_mode(extra_config);

    if let Some(api_key) = api_key_secret
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
    {
        entries.push(("GEMINI_API_KEY".to_string(), api_key.clone()));
        if env_mode == "legacy" {
            entries.push(("GOOGLE_API_KEY".to_string(), api_key));
        }
    }

    if let Some(base_url) = provider
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
    {
        entries.push(("GOOGLE_GEMINI_BASE_URL".to_string(), base_url));
    }
    if let Some(project_id) = get_extra_string(extra_config, "projectId") {
        entries.push(("GOOGLE_CLOUD_PROJECT".to_string(), project_id));
    }
    if let Some(model) = resolve_tool_model(
        extra_config,
        "gemini_cli",
        Some(provider.model_name.as_str()),
    ) {
        entries.push(("GEMINI_MODEL".to_string(), model));
    }

    entries
}

fn is_managed_env_assignment(line: &str, managed_keys: &[&str]) -> bool {
    let trimmed = line.trim_start();
    let without_export = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let Some((key, _)) = without_export.split_once('=') else {
        return false;
    };
    let key = key.trim();
    managed_keys
        .iter()
        .any(|managed_key| key.eq_ignore_ascii_case(managed_key))
}

fn escape_env_value(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }
    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '#' | '"' | '\\'))
    {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn render_env_file_with_managed_keys(
    existing_content: Option<&str>,
    managed_keys: &[&str],
    entries: &[(String, String)],
) -> String {
    let mut preserved_lines = existing_content
        .unwrap_or_default()
        .lines()
        .filter(|line| !is_managed_env_assignment(line, managed_keys))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    while preserved_lines
        .last()
        .map(|line| line.trim().is_empty())
        .unwrap_or(false)
    {
        preserved_lines.pop();
    }

    if !entries.is_empty() && !preserved_lines.is_empty() {
        preserved_lines.push(String::new());
    }

    preserved_lines.extend(
        entries
            .iter()
            .map(|(key, value)| format!("{key}={}", escape_env_value(value))),
    );

    if preserved_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", preserved_lines.join("\n"))
    }
}

fn render_gemini_env_file(
    existing_content: Option<&str>,
    provider: &ProviderConfig,
    extra_config: &Value,
    api_key_secret: Option<&str>,
) -> String {
    let entries = build_gemini_env_entries(provider, extra_config, api_key_secret);
    render_env_file_with_managed_keys(existing_content, gemini_managed_env_keys(), &entries)
}

fn render_codex_config(
    existing_content: Option<&str>,
    p: &ProviderConfig,
    mcps: &[McpServer],
) -> Result<String, String> {
    let extra_config = parse_provider_extra_config(p.extra_config.as_deref());
    let mut doc = parse_codex_document(existing_content);
    doc["model_provider"] = value(CODEX_MANAGED_PROVIDER_KEY);
    doc["model"] = value(
        resolve_tool_model(&extra_config, "codex", Some(p.model_name.as_str()))
            .unwrap_or_else(|| "gpt-4o".to_string()),
    );
    doc["model_reasoning_effort"] = value(
        get_tool_config_string(&extra_config, "codex", "reasoningEffort")
            .unwrap_or_else(|| "high".to_string()),
    );
    doc["disable_response_storage"] = value(true);

    let should_reset_model_providers = doc
        .as_table()
        .get("model_providers")
        .map(|item| !item.is_table())
        .unwrap_or(true);
    if should_reset_model_providers {
        doc.as_table_mut()
            .insert("model_providers", Item::Table(Table::new()));
    }

    let model_providers = doc
        .as_table_mut()
        .get_mut("model_providers")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| "Codex config model_providers 格式无效".to_string())?;
    model_providers.remove(CODEX_LEGACY_PROVIDER_KEY);

    let mut provider_table = Table::new();
    provider_table["name"] = value(p.name.as_str());
    provider_table["base_url"] = value(normalize_codex_base_url(p.base_url.as_deref()));
    provider_table["wire_api"] = value("responses");
    provider_table["requires_openai_auth"] = value(true);
    model_providers[CODEX_MANAGED_PROVIDER_KEY] = Item::Table(provider_table);

    let mcp_servers = render_codex_mcp_servers(mcps);
    if mcp_servers.is_empty() {
        doc.as_table_mut().remove("mcp_servers");
    } else {
        doc["mcp_servers"] = Item::Table(mcp_servers);
    }

    Ok(doc.to_string())
}

fn parse_codex_document(existing_content: Option<&str>) -> DocumentMut {
    existing_content
        .and_then(|raw| raw.parse::<DocumentMut>().ok())
        .unwrap_or_default()
}

fn normalize_codex_base_url(raw: Option<&str>) -> String {
    let base_url = raw.unwrap_or("https://api.openai.com/v1").trim();
    if base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{}/v1", base_url.trim_end_matches('/'))
    }
}

fn render_codex_mcp_servers(mcps: &[McpServer]) -> Table {
    let mut mcp_servers = Table::new();

    for mcp in mcps {
        if !mcp.is_active || !mcp.parsed_tool_targets().contains(&ToolTarget::Codex) {
            continue;
        }

        let mut server_tbl = Table::new();
        let cmd = mcp.command.trim().to_string();
        let server_type = if cmd.starts_with("http://") || cmd.starts_with("https://") {
            "http"
        } else {
            "stdio"
        };

        server_tbl["type"] = value(server_type);
        if server_type == "stdio" {
            server_tbl["command"] = value(cmd);

            if let Some(args) = &mcp.args {
                if let Ok(args_arr) = serde_json::from_str::<Vec<String>>(args) {
                    let mut arr = toml_edit::Array::default();
                    for arg in args_arr {
                        arr.push(arg);
                    }
                    if !arr.is_empty() {
                        server_tbl["args"] = Item::Value(TomlValue::Array(arr));
                    }
                }
            }

            if let Some(env) = &mcp.env {
                if let Ok(env_map) =
                    serde_json::from_str::<std::collections::HashMap<String, String>>(env)
                {
                    let mut env_tbl = Table::new();
                    for (k, v) in env_map {
                        env_tbl[&k] = value(v);
                    }
                    if !env_tbl.is_empty() {
                        server_tbl["env"] = Item::Table(env_tbl);
                    }
                }
            }
        } else {
            server_tbl["url"] = value(cmd);
        }

        mcp_servers[&mcp.name] = Item::Table(server_tbl);
    }

    mcp_servers
}

#[cfg(test)]
mod tests {
    use super::{
        build_claude_env, build_codex_auth_json, build_gemini_settings,
        parse_provider_extra_config, render_codex_config, render_gemini_env_file,
        CODEX_MANAGED_PROVIDER_KEY,
    };
    use crate::models::{McpServer, Platform, ProviderConfig};
    use chrono::Utc;
    use serde_json::json;

    fn sample_provider() -> ProviderConfig {
        ProviderConfig {
            id: "provider-1".to_string(),
            name: "AI Singularity Network".to_string(),
            platform: Platform::Custom,
            category: None,
            base_url: Some("https://api.aisingularity.com".to_string()),
            api_key_id: Some("key-1".to_string()),
            model_name: "gpt-5.4".to_string(),
            is_active: true,
            tool_targets: Some("[\"codex\"]".to_string()),
            icon: None,
            icon_color: None,
            website_url: None,
            api_key_url: None,
            notes: None,
            extra_config: Some(
                json!({
                    "apiKeyField": "ANTHROPIC_AUTH_TOKEN",
                    "envInjection": "legacy",
                    "projectId": "vertex-demo-123",
                    "tool_configs": {
                        "claude_code": {
                            "model": "claude-opus-4-5",
                            "reasoningModel": "claude-3-7-sonnet-20250219",
                            "haikuModel": "claude-3-5-haiku-20241022",
                            "sonnetModel": "claude-sonnet-4-5",
                            "opusModel": "claude-opus-4-5"
                        },
                        "codex": {
                            "model": "gpt-5.4-codex",
                            "reasoningEffort": "medium"
                        },
                        "gemini_cli": {
                            "model": "gemini-2.5-pro"
                        },
                        "open_code": {
                            "model": "gpt-4.1"
                        },
                        "open_claw": {
                            "model": "gpt-4.1-mini"
                        },
                        "aider": {
                            "model": "claude-3-7-sonnet-20250219"
                        }
                    }
                })
                .to_string(),
            ),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_mcp() -> McpServer {
        McpServer {
            id: "mcp-1".to_string(),
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: Some("[\"-y\",\"@modelcontextprotocol/server-filesystem\"]".to_string()),
            env: Some("{\"ROOT\":\"D:/Code\"}".to_string()),
            description: None,
            is_active: true,
            tool_targets: Some("[\"codex\"]".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn build_codex_auth_json_writes_real_api_key() {
        let auth = build_codex_auth_json(" sk-test-123 ");
        assert_eq!(auth["OPENAI_API_KEY"], "sk-test-123");
        assert_eq!(auth.as_object().map(|obj| obj.len()), Some(1));
    }

    #[test]
    fn build_claude_env_applies_tool_overrides_and_auth_field() {
        let env = build_claude_env(
            json!({
                "SHOULD_KEEP": "yes",
                "ANTHROPIC_API_KEY": "old-key"
            }),
            &sample_provider(),
            Some("sk-anthropic-123"),
        );

        assert_eq!(env["SHOULD_KEEP"], json!("yes"));
        assert_eq!(env["ANTHROPIC_AUTH_TOKEN"], json!("sk-anthropic-123"));
        assert!(env.get("ANTHROPIC_API_KEY").is_none());
        assert_eq!(env["ANTHROPIC_MODEL"], json!("claude-opus-4-5"));
        assert_eq!(
            env["ANTHROPIC_REASONING_MODEL"],
            json!("claude-3-7-sonnet-20250219")
        );
        assert_eq!(
            env["ANTHROPIC_DEFAULT_HAIKU_MODEL"],
            json!("claude-3-5-haiku-20241022")
        );
    }

    #[test]
    fn build_gemini_settings_preserves_existing_shape_and_switches_to_api_key_auth() {
        let provider = sample_provider();
        let extra_config = parse_provider_extra_config(provider.extra_config.as_deref());
        let settings = build_gemini_settings(
            json!({
                "theme": "dark",
                "security": {
                    "auth": {
                        "selectedType": "oauth-personal",
                        "keepMe": true
                    }
                }
            }),
            &provider,
            &extra_config,
        );

        assert_eq!(settings["theme"], json!("dark"));
        assert_eq!(
            settings["security"]["auth"]["selectedType"],
            json!("gemini-api-key")
        );
        assert_eq!(settings["security"]["auth"]["keepMe"], json!(true));
        assert_eq!(
            settings["GOOGLE_GEMINI_BASE_URL"],
            json!("https://api.aisingularity.com")
        );
        assert_eq!(settings["model"], json!("gemini-2.5-pro"));
        assert_eq!(settings["projectId"], json!("vertex-demo-123"));
    }

    #[test]
    fn render_gemini_env_file_preserves_unmanaged_lines_and_rewrites_managed_keys() {
        let provider = sample_provider();
        let extra_config = parse_provider_extra_config(provider.extra_config.as_deref());
        let rendered = render_gemini_env_file(
            Some(
                "CUSTOM_KEEP=1\n\
                 GOOGLE_API_KEY=old-google\n\
                 export GEMINI_API_KEY=old-gemini\n\
                 GEMINI_MODEL=old-model\n",
            ),
            &provider,
            &extra_config,
            Some(" sk-gemini-123 "),
        );

        assert!(rendered.contains("CUSTOM_KEEP=1"));
        assert!(rendered.contains("GEMINI_API_KEY=sk-gemini-123"));
        assert!(rendered.contains("GOOGLE_API_KEY=sk-gemini-123"));
        assert!(rendered.contains("GOOGLE_GEMINI_BASE_URL=https://api.aisingularity.com"));
        assert!(rendered.contains("GOOGLE_CLOUD_PROJECT=vertex-demo-123"));
        assert!(rendered.contains("GEMINI_MODEL=gemini-2.5-pro"));
        assert!(!rendered.contains("old-google"));
        assert!(!rendered.contains("old-gemini"));
        assert!(!rendered.contains("old-model"));
    }

    #[test]
    fn render_gemini_env_file_standard_mode_removes_legacy_google_api_key() {
        let mut provider = sample_provider();
        provider.extra_config = Some(
            json!({
                "envInjection": "standard",
                "tool_configs": {
                    "gemini_cli": {
                        "model": "gemini-2.5-flash"
                    }
                }
            })
            .to_string(),
        );
        let extra_config = parse_provider_extra_config(provider.extra_config.as_deref());
        let rendered = render_gemini_env_file(
            Some("GOOGLE_API_KEY=old-google\nCUSTOM=ok\n"),
            &provider,
            &extra_config,
            Some("sk-gemini-456"),
        );

        assert!(rendered.contains("CUSTOM=ok"));
        assert!(rendered.contains("GEMINI_API_KEY=sk-gemini-456"));
        assert!(rendered.contains("GEMINI_MODEL=gemini-2.5-flash"));
        assert!(!rendered.contains("GOOGLE_API_KEY="));
    }

    #[test]
    fn render_codex_config_preserves_unmanaged_sections() {
        let existing = r#"
model_provider = "custom"
model = "old-model"
model_reasoning_effort = "high"

[windows]
sandbox = "elevated"

[projects.'D:\Code\Tauri\AI Singularity']
trust_level = "trusted"
"#;

        let rendered =
            render_codex_config(Some(existing), &sample_provider(), &[sample_mcp()]).unwrap();

        assert!(rendered.contains("model_provider = \"ai_singularity\""));
        assert!(rendered.contains("model = \"gpt-5.4-codex\""));
        assert!(rendered.contains("model_reasoning_effort = \"medium\""));
        assert!(rendered.contains("[windows]"));
        assert!(rendered.contains("sandbox = \"elevated\""));
        assert!(rendered.contains("[projects.'D:\\Code\\Tauri\\AI Singularity']"));
        assert!(rendered.contains("[model_providers.ai_singularity]"));
        assert!(rendered.contains("base_url = \"https://api.aisingularity.com/v1\""));
        assert!(rendered.contains("[mcp_servers.filesystem]"));
    }

    #[test]
    fn render_codex_config_removes_legacy_managed_provider() {
        let existing = r#"
[model_providers.newapi]
name = "old"
base_url = "https://old.example.com/v1"
"#;

        let rendered = render_codex_config(Some(existing), &sample_provider(), &[]).unwrap();

        assert!(!rendered.contains("[model_providers.newapi]"));
        assert!(rendered.contains(&format!("[model_providers.{}]", CODEX_MANAGED_PROVIDER_KEY)));
        assert!(!rendered.contains("[mcp_servers."));
    }
}
