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
