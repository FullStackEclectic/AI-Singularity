use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================
// Platform（API 供应商平台）
// ============================================================

/// AI 平台枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Aliyun,    // 阿里云百炼
    Bytedance, // 字节豆包
    Moonshot,  // Kimi
    Zhipu,     // 智谱 GLM
    MiniMax,
    StepFun,
    AwsBedrock,
    NvidiaNim,
    AzureOpenAI,
    SiliconFlow,
    OpenRouter,
    Groq,
    Mistral,
    XAi,
    Cohere,
    Perplexity,
    TogetherAi,
    Ollama,
    HuggingFace,
    Replicate,
    Copilot,  // GitHub Copilot OAuth
    Auth0IDE, // 高阶白嫖设备指纹伪装池
    Custom,   // 自定义 OpenAI 兼容
}

impl Platform {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            Platform::OpenAI => "OpenAI",
            Platform::Anthropic => "Anthropic",
            Platform::Gemini => "Google Gemini",
            Platform::DeepSeek => "DeepSeek",
            Platform::Aliyun => "阿里云百炼",
            Platform::Bytedance => "字节豆包 (DouBao)",
            Platform::Moonshot => "Moonshot (Kimi)",
            Platform::Zhipu => "智谱 GLM / Z.ai",
            Platform::MiniMax => "MiniMax",
            Platform::StepFun => "StepFun",
            Platform::AwsBedrock => "AWS Bedrock",
            Platform::NvidiaNim => "NVIDIA NIM",
            Platform::AzureOpenAI => "Azure OpenAI",
            Platform::SiliconFlow => "SiliconFlow",
            Platform::OpenRouter => "OpenRouter",
            Platform::Groq => "Groq",
            Platform::Mistral => "Mistral AI",
            Platform::XAi => "xAI (Grok)",
            Platform::Cohere => "Cohere",
            Platform::Perplexity => "Perplexity",
            Platform::TogetherAi => "Together AI",
            Platform::Ollama => "Ollama",
            Platform::HuggingFace => "Hugging Face",
            Platform::Replicate => "Replicate",
            Platform::Copilot => "GitHub Copilot",
            Platform::Auth0IDE => "超级白嫖指纹池",
            Platform::Custom => "自定义接口",
        }
    }
}

// ============================================================
// ToolTarget（目标 AI 编码工具，用于多工具同步）
// ============================================================

/// 支持同步配置的 AI 编码工具
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ToolTarget {
    ClaudeCode,
    Codex,
    GeminiCli,
    OpenCode,
    OpenClaw,
    Aider,
}

impl ToolTarget {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            ToolTarget::ClaudeCode => "Claude Code",
            ToolTarget::Codex => "OpenAI Codex",
            ToolTarget::GeminiCli => "Gemini CLI",
            ToolTarget::OpenCode => "OpenCode",
            ToolTarget::OpenClaw => "OpenClaw",
            ToolTarget::Aider => "Aider",
        }
    }

    /// 配置文件路径描述（用于 UI 提示）
    #[allow(dead_code)]
    pub fn config_path_hint(&self) -> &str {
        match self {
            ToolTarget::ClaudeCode => "~/.claude/settings.json",
            ToolTarget::Codex => "~/.codex/config.toml",
            ToolTarget::GeminiCli => "~/.gemini/settings.json",
            ToolTarget::OpenCode => "~/.config/opencode/opencode.json",
            ToolTarget::OpenClaw => "~/.openclaw/config.json",
            ToolTarget::Aider => "~/.aider.conf.yml",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> Vec<ToolTarget> {
        vec![
            ToolTarget::ClaudeCode,
            ToolTarget::Codex,
            ToolTarget::GeminiCli,
            ToolTarget::OpenCode,
            ToolTarget::OpenClaw,
            ToolTarget::Aider,
        ]
    }
}

// ============================================================
// Provider 配置（核心模型）
// ============================================================

/// Provider 预设分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCategory {
    Official,      // 官方（Anthropic、OpenAI、Google）
    CnOfficial,    // 国内大模型官方
    CloudProvider, // 云厂商（AWS、Azure、NVIDIA）
    Aggregator,    // 聚合平台（OpenRouter、SiliconFlow）
    ThirdParty,    // 第三方中继
    Custom,        // 用户自定义
}

impl ProviderCategory {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            ProviderCategory::Official => "官方",
            ProviderCategory::CnOfficial => "国内大模型",
            ProviderCategory::CloudProvider => "云厂商",
            ProviderCategory::Aggregator => "聚合平台",
            ProviderCategory::ThirdParty => "第三方中继",
            ProviderCategory::Custom => "自定义",
        }
    }
}

// ============================================================
// Model Catalog
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub context_length: Option<u32>,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub input_price_per_1m: Option<f64>,
    pub output_price_per_1m: Option<f64>,
    pub is_available: bool,
}

/// Provider 配置（支持多工具同步）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    /// 平台类型（供应商身份）
    pub platform: Platform,
    /// 分类（用于 UI 展示分组）
    #[serde(default)]
    pub category: Option<ProviderCategory>,
    /// API Base URL
    pub base_url: Option<String>,
    /// 加密存储的 API Key ID（关联 api_keys 表）
    pub api_key_id: Option<String>,
    /// 默认模型名称
    pub model_name: String,
    /// 是否启用（当前激活的 provider）
    pub is_active: bool,
    /// 同步到哪些工具（JSON 数组，如 ["claude_code","codex"]）
    /// 空数组 = 不同步到任何工具；None = 仅 claude_code（向后兼容）
    pub tool_targets: Option<String>,
    /// 图标标识符（对应前端 BrandIcon 组件）
    pub icon: Option<String>,
    /// 图标颜色（Hex）
    pub icon_color: Option<String>,
    /// 网站链接
    pub website_url: Option<String>,
    /// API Key 申请链接
    pub api_key_url: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 扩展配置（JSON，存储各工具特定参数）
    pub extra_config: Option<String>,
    /// 列表排序优先级，默认为0，数值越小越靠前
    #[serde(default)]
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProviderConfig {
    /// 解析 tool_targets 字段为枚举列表
    pub fn parsed_tool_targets(&self) -> Vec<ToolTarget> {
        match &self.tool_targets {
            None => vec![ToolTarget::ClaudeCode], // 向后兼容
            Some(s) if s.is_empty() || s == "[]" => vec![],
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
        }
    }

    /// 判断是否同步到指定工具
    pub fn syncs_to(&self, tool: &ToolTarget) -> bool {
        self.parsed_tool_targets().contains(tool)
    }
}

// ============================================================
// API Key
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub base_url: Option<String>,
    pub key_preview: String, // 仅展示前8位 + "..."
    pub status: KeyStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_checked_at: Option<DateTime<Utc>>,
    /// 轮询优先级：数值越大越优先（默认 100）
    #[serde(default = "default_priority")]
    pub priority: i64,
    /// 用户自定义标签（JSON 字符串存储，如 ["vip","生产"]）
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

fn default_priority() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KeyStatus {
    Unknown,
    Valid,
    Invalid,
    Expired,
    Banned,    // 403
    RateLimit, // 429
}

// ============================================================
// 余额快照（Balance Tracker）
// ============================================================

/// 余额快照，用于时序趋势
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub balance_usd: Option<f64>,
    pub balance_cny: Option<f64>,
    pub quota_remaining: Option<f64>,
    pub quota_unit: Option<String>, // "tokens" | "usd" | "requests"
    pub quota_reset_at: Option<DateTime<Utc>>,
    pub snapped_at: DateTime<Utc>,
}

/// 余额汇总（Dashboard 展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSummary {
    pub provider_id: String,
    pub provider_name: String,
    pub platform: String,
    pub latest_balance_usd: Option<f64>,
    pub latest_balance_cny: Option<f64>,
    pub quota_remaining: Option<f64>,
    pub quota_unit: Option<String>,
    pub quota_reset_at: Option<DateTime<Utc>>,
    pub last_updated: Option<DateTime<Utc>>,
    pub low_balance_alert: bool,
}

// ============================================================
// 余额预测 (Burn Rate Forecast)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRateForecast {
    pub provider_id: String,
    pub daily_burn_rate_usd: Option<f64>,
    pub daily_burn_rate_cny: Option<f64>,
    pub estimated_depletion_date: Option<DateTime<Utc>>,
    pub is_at_risk: bool,
    pub next_reset_at: Option<DateTime<Utc>>,
    pub reset_cycle: String, // "monthly", "daily", "prepaid_none", "unknown"
}

// ============================================================
// MCP Server
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Option<String>, // JSON array
    pub env: Option<String>,  // JSON map
    pub description: Option<String>,
    pub is_active: bool,
    /// 同步到哪些工具（JSON 数组）
    pub tool_targets: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl McpServer {
    pub fn parsed_tool_targets(&self) -> Vec<ToolTarget> {
        match &self.tool_targets {
            None => vec![ToolTarget::ClaudeCode],
            Some(s) if s.is_empty() || s == "[]" => vec![],
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
        }
    }
}

// ============================================================
// Prompt
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub target_file: String, // e.g., CLAUDE.md / .aider.conf.yml / system_prompt
    pub content: String,
    pub is_active: bool,
    /// 同步到哪些工具（JSON 数组）
    pub tool_targets: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PromptConfig {
    pub fn parsed_tool_targets(&self) -> Vec<ToolTarget> {
        match &self.tool_targets {
            None => vec![], // 默认不覆盖所有工具
            Some(s) if s.is_empty() || s == "[]" => vec![],
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
        }
    }
}

// ============================================================
// Alert
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertItem {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub platform: Option<String>,
    pub key_id: Option<String>,
}

// ============================================================
// SpeedTest & Stream Check
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub platform: String,
    pub endpoint: String,
    pub latency_ms: Option<u64>,
    pub status: String, // "ok" | "timeout" | "error"
}

/// 流式连通性检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // 前端使用驼峰命名
pub struct StreamCheckResult {
    pub status: String, // "operational", "degraded", "failed"
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<u64>,
    pub model_used: String,
}

// ============================================================
// Token 审计 (Usage Stats)
// ============================================================

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    pub id: String,
    pub key_id: String,
    pub platform: String,
    pub model_name: String,
    /// 客户端请求头标识 (如 Claude Code, Aider 等)
    pub client_app: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub created_at: DateTime<Utc>,
}

// ============================================================
// 降维打击武器库：防封杀硬核指纹架构与池化 Token 网关 (IDE)
// ============================================================

/// 物理机硬件环境拟态指纹
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DeviceProfile {
    pub machine_id: String,
    pub mac_machine_id: String,
    pub dev_device_id: String,
    pub sqm_id: String,
}

/// Token 池中账号的当前实战健康度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    /// 活跃运作中
    Active,
    /// 令牌已物理过期，需强制下线
    Expired,
    /// 被判定滥用、强制403锁定截杀
    Forbidden,
    /// 短期并发过高，被关进小黑屋
    RateLimited,
    /// 未知
    Unknown,
}

/// 支持刷新续期的 OAuth 令牌环
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub updated_at: DateTime<Utc>,
}

/// 指纹型高级池化子网账户实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeAccount {
    /// 全局唯一标号（多数场景通过 refresh_token 或者邮箱做哈希去重）
    pub id: String,
    pub email: String,
    /// 通道所属应用大类，限制本指纹针对哪个平台池（如：ClaudeCode, Cursor...）
    pub origin_platform: String,
    pub token: OAuthToken,
    pub status: AccountStatus,
    /// 是否在轮询池里处于被干掉的状态原因脱水
    pub disabled_reason: Option<String>,
    /// 是否被人为强制禁用代理转发
    pub is_proxy_disabled: bool,
    /// 大盘指标记录
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,

    /// 核武器：本机硬件码伪装。此项为 Some 时，底层转发网关将剥离原生请求的 Header 和数据包特征，并强行植入其对应的机器码，实现“白嫖免死金牌”。
    pub device_profile: Option<DeviceProfile>,

    /// 当该账号支持高阶的限流逻辑（如 Claude 的按次限购）时，脱水保存其配额额度。
    pub quota_json: Option<String>,

    /// Gemini 等平台使用的项目上下文
    pub project_id: Option<String>,

    /// 平台特有的扩展元数据（如 Codex auth.json 中的 id_token/account_id）
    pub meta_json: Option<String>,

    /// 用户自定义备注名（优先用于 UI 展示）
    pub label: Option<String>,

    /// 用户自定义标签（JSON 字符串存储，如 ["vip","生产"]）
    #[serde(default)]
    pub tags: Vec<String>,
}

// ============================================================
// SaaS User Token (用于向下级分发网关使用权的实体)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserToken {
    pub id: String,
    pub token: String,
    pub username: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub expires_type: String, // "never", "relative", "absolute"
    pub expires_at: Option<i64>,
    pub max_ips: i64,
    pub curfew_start: Option<String>,
    pub curfew_end: Option<String>,
    pub total_requests: i64,
    pub total_tokens_used: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenScope {
    #[serde(default = "default_scope")]
    pub scope: String, // "global" | "channel" | "tag" | "single"
    pub desc: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub single_account: Option<String>,
}

fn default_scope() -> String {
    "global".to_string()
}

impl UserToken {
    pub fn parse_scope(&self) -> TokenScope {
        if let Some(desc) = &self.description {
            if desc.starts_with('{') && desc.contains("\"scope\"") {
                if let Ok(scope) = serde_json::from_str::<TokenScope>(desc) {
                    return scope;
                }
            }
        }
        TokenScope {
            scope: "global".to_string(),
            desc: self.description.clone(),
            channels: vec![],
            tags: vec![],
            single_account: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserTokenReq {
    pub username: String,
    pub description: Option<String>,
    pub expires_type: String,
    pub expires_at: Option<i64>,
    pub max_ips: i64,
    pub curfew_start: Option<String>,
    pub curfew_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserTokenReq {
    pub id: String,
    pub username: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub expires_type: Option<String>,
    pub expires_at: Option<i64>,
    pub max_ips: Option<i64>,
    pub curfew_start: Option<String>,
    pub curfew_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTokenSummary {
    pub total_tokens: i64,
    pub active_tokens: i64,
    pub total_users: i64,
    pub today_requests: i64,
}

// ============================================================
// Proxy Engine Configuration settings
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulingConfig {
    pub mode: String, // "Balance" | "Priority" | "Latency"
    pub max_wait_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub backoff_steps: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedThinkingConfig {
    pub enabled: bool,
    pub compression_threshold: f64,
    pub budget_limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    pub scheduling: SchedulingConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub advanced_thinking: AdvancedThinkingConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            scheduling: SchedulingConfig {
                mode: "Balance".to_string(),
                max_wait_secs: 60,
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                backoff_steps: vec![60, 120, 300, 600],
            },
            advanced_thinking: AdvancedThinkingConfig {
                enabled: false,
                compression_threshold: 0.65,
                budget_limit: 4096,
            },
        }
    }
}

// ============================================================
// Security Firewall & Domain Objects
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAccessLog {
    pub id: String,
    pub ip_address: String,
    pub endpoint: String,
    pub token_id: Option<String>,
    pub action_taken: String, // "allow", "deny", "rate_limit", "blacklisted"
    pub reason: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpRule {
    pub id: String,
    pub ip_cidr: String,
    pub rule_type: String, // "blacklist" | "whitelist"
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
}
