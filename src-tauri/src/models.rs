use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    AwsBedrock,
    NvidiaNim,
    Custom, // 自定义 OpenAI 兼容
}

impl Platform {
    pub fn display_name(&self) -> &str {
        match self {
            Platform::OpenAI => "OpenAI",
            Platform::Anthropic => "Anthropic (Claude)",
            Platform::Gemini => "Google Gemini",
            Platform::DeepSeek => "DeepSeek",
            Platform::Aliyun => "阿里云百炼",
            Platform::Bytedance => "字节豆包",
            Platform::Moonshot => "Moonshot (Kimi)",
            Platform::Zhipu => "智谱 GLM",
            Platform::AwsBedrock => "AWS Bedrock",
            Platform::NvidiaNim => "NVIDIA NIM",
            Platform::Custom => "自定义接口",
        }
    }
}

/// API Key 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub base_url: Option<String>, // 自定义接口 URL
    pub key_preview: String,      // 仅展示前8位 + "..."
    pub status: KeyStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_checked_at: Option<DateTime<Utc>>,
}

/// API Key 状态
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

/// 余额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub key_id: String,
    pub platform: Platform,
    pub balance_usd: Option<f64>,
    pub balance_cny: Option<f64>,
    pub total_usage_usd: Option<f64>,
    pub quota_remaining: Option<f64>, // 剩余配额（tokens 或 美元）
    pub quota_reset_at: Option<DateTime<Utc>>,
    pub synced_at: DateTime<Utc>,
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub context_length: Option<u64>,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub input_price_per_1m: Option<f64>, // 每百万 token 价格（USD）
    pub output_price_per_1m: Option<f64>,
    pub is_available: bool,
}

/// AI 辅助工具枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AiTool {
    ClaudeCode,
    Aider,
    Codex,
    GeminiCli,
    OpenCode,
}

impl AiTool {
    pub fn display_name(&self) -> &str {
        match self {
            AiTool::ClaudeCode => "Claude Code",
            AiTool::Aider => "Aider",
            AiTool::Codex => "Codex",
            AiTool::GeminiCli => "Gemini CLI",
            AiTool::OpenCode => "OpenCode",
        }
    }
}

/// Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub ai_tool: AiTool,
    pub platform: Platform,
    pub base_url: Option<String>,
    pub api_key_id: Option<String>,
    pub model_name: String,
    pub custom_config: Option<String>, // JSON string map
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MCP Server 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Option<String>, // JSON array
    pub env: Option<String>,  // JSON map
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Prompt 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub id: String,
    pub name: String,
    pub target_file: String, // e.g., CLAUDE.md
    pub content: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 告警等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// 告警条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertItem {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub platform: Option<String>,
    pub key_id: Option<String>,
}

/// 延迟测速结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub platform: String,
    pub endpoint: String,
    pub latency_ms: Option<u64>,
    pub status: String, // "ok", "timeout", "error"
}
