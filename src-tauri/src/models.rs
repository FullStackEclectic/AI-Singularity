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
    Aliyun,      // 阿里云百炼
    Bytedance,   // 字节豆包
    Moonshot,    // Kimi
    Zhipu,       // 智谱 GLM
    MiniMax,
    StepFun,
    AwsBedrock,
    NvidiaNim,
    AzureOpenAI,
    SiliconFlow,
    OpenRouter,
    Copilot,     // GitHub Copilot OAuth
    Custom,      // 自定义 OpenAI 兼容
}

impl Platform {
    pub fn display_name(&self) -> &str {
        match self {
            Platform::OpenAI     => "OpenAI",
            Platform::Anthropic  => "Anthropic",
            Platform::Gemini     => "Google Gemini",
            Platform::DeepSeek   => "DeepSeek",
            Platform::Aliyun     => "阿里云百炼",
            Platform::Bytedance  => "字节豆包 (DouBao)",
            Platform::Moonshot   => "Moonshot (Kimi)",
            Platform::Zhipu      => "智谱 GLM / Z.ai",
            Platform::MiniMax    => "MiniMax",
            Platform::StepFun    => "StepFun",
            Platform::AwsBedrock => "AWS Bedrock",
            Platform::NvidiaNim  => "NVIDIA NIM",
            Platform::AzureOpenAI => "Azure OpenAI",
            Platform::SiliconFlow => "SiliconFlow",
            Platform::OpenRouter  => "OpenRouter",
            Platform::Copilot    => "GitHub Copilot",
            Platform::Custom     => "自定义接口",
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
    pub fn display_name(&self) -> &str {
        match self {
            ToolTarget::ClaudeCode => "Claude Code",
            ToolTarget::Codex      => "OpenAI Codex",
            ToolTarget::GeminiCli  => "Gemini CLI",
            ToolTarget::OpenCode   => "OpenCode",
            ToolTarget::OpenClaw   => "OpenClaw",
            ToolTarget::Aider      => "Aider",
        }
    }

    /// 配置文件路径描述（用于 UI 提示）
    pub fn config_path_hint(&self) -> &str {
        match self {
            ToolTarget::ClaudeCode => "~/.claude.json",
            ToolTarget::Codex      => "~/.codex/config.toml",
            ToolTarget::GeminiCli  => "~/.gemini/settings.json",
            ToolTarget::OpenCode   => "~/.config/opencode/opencode.json",
            ToolTarget::OpenClaw   => "~/.openclaw/config.json",
            ToolTarget::Aider      => "~/.aider.conf.yml",
        }
    }

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
    Official,       // 官方（Anthropic、OpenAI、Google）
    CnOfficial,     // 国内大模型官方
    CloudProvider,  // 云厂商（AWS、Azure、NVIDIA）
    Aggregator,     // 聚合平台（OpenRouter、SiliconFlow）
    ThirdParty,     // 第三方中继
    Custom,         // 用户自定义
}

impl ProviderCategory {
    pub fn display_name(&self) -> &str {
        match self {
            ProviderCategory::Official      => "官方",
            ProviderCategory::CnOfficial    => "国内大模型",
            ProviderCategory::CloudProvider => "云厂商",
            ProviderCategory::Aggregator    => "聚合平台",
            ProviderCategory::ThirdParty    => "第三方中继",
            ProviderCategory::Custom        => "自定义",
        }
    }
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
    pub args: Option<String>,  // JSON array
    pub env: Option<String>,   // JSON map
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
// SpeedTest
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub platform: String,
    pub endpoint: String,
    pub latency_ms: Option<u64>,
    pub status: String, // "ok" | "timeout" | "error"
}
