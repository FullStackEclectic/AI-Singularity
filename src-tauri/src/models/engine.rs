use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulingConfig {
    pub mode: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAccessLog {
    pub id: String,
    pub ip_address: String,
    pub endpoint: String,
    pub token_id: Option<String>,
    pub action_taken: String,
    pub reason: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpRule {
    pub id: String,
    pub ip_cidr: String,
    pub rule_type: String,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
}
