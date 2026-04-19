use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Aliyun,
    Bytedance,
    Moonshot,
    Zhipu,
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
    Copilot,
    Auth0IDE,
    Custom,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCategory {
    Official,
    CnOfficial,
    CloudProvider,
    Aggregator,
    ThirdParty,
    Custom,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    #[serde(default)]
    pub category: Option<ProviderCategory>,
    pub base_url: Option<String>,
    pub api_key_id: Option<String>,
    pub model_name: String,
    pub is_active: bool,
    pub tool_targets: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub website_url: Option<String>,
    pub api_key_url: Option<String>,
    pub notes: Option<String>,
    pub extra_config: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProviderConfig {
    pub fn parsed_tool_targets(&self) -> Vec<ToolTarget> {
        match &self.tool_targets {
            None => vec![ToolTarget::ClaudeCode],
            Some(s) if s.is_empty() || s == "[]" => vec![],
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
        }
    }

    pub fn syncs_to(&self, tool: &ToolTarget) -> bool {
        self.parsed_tool_targets().contains(tool)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub base_url: Option<String>,
    pub key_preview: String,
    pub status: KeyStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_checked_at: Option<DateTime<Utc>>,
    #[serde(default = "default_priority")]
    pub priority: i64,
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
    Banned,
    RateLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub balance_usd: Option<f64>,
    pub balance_cny: Option<f64>,
    pub quota_remaining: Option<f64>,
    pub quota_unit: Option<String>,
    pub quota_reset_at: Option<DateTime<Utc>>,
    pub snapped_at: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRateForecast {
    pub provider_id: String,
    pub daily_burn_rate_usd: Option<f64>,
    pub daily_burn_rate_cny: Option<f64>,
    pub estimated_depletion_date: Option<DateTime<Utc>>,
    pub is_at_risk: bool,
    pub next_reset_at: Option<DateTime<Utc>>,
    pub reset_cycle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub target_file: String,
    pub content: String,
    pub is_active: bool,
    pub tool_targets: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PromptConfig {
    pub fn parsed_tool_targets(&self) -> Vec<ToolTarget> {
        match &self.tool_targets {
            None => vec![],
            Some(s) if s.is_empty() || s == "[]" => vec![],
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub platform: String,
    pub endpoint: String,
    pub latency_ms: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamCheckResult {
    pub status: String,
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<u64>,
    pub model_used: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    pub id: String,
    pub key_id: String,
    pub platform: String,
    pub model_name: String,
    pub client_app: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub created_at: DateTime<Utc>,
}
