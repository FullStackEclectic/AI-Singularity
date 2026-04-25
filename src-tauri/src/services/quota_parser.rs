use crate::models::IdeAccount;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ModelQuota {
    pub name: String,
    pub percentage: i32,
}

#[derive(Debug, Clone)]
pub struct QuotaSummary {
    pub is_forbidden: bool,
    pub models: Vec<ModelQuota>,
}

impl QuotaSummary {
    /// 兼容多种 quota_json 形态：
    /// - cockpit 风格：{ "is_forbidden": bool, "models": [{ "name": str, "percentage": i32 }] }
    /// - codex 风格：{ "hourly_percentage": i32, "weekly_percentage": i32, ... }
    /// - gemini 风格：{ "project_id": ..., "quota": ... }
    pub fn parse(account: &IdeAccount) -> Option<Self> {
        let raw = account.quota_json.as_deref()?;
        let value: Value = serde_json::from_str(raw).ok()?;

        if let Some(models_array) = value.get("models").and_then(|v| v.as_array()) {
            let models: Vec<ModelQuota> = models_array
                .iter()
                .filter_map(|m| {
                    let name = m.get("name")?.as_str()?.to_string();
                    let percentage = m.get("percentage")?.as_i64()? as i32;
                    Some(ModelQuota { name, percentage })
                })
                .collect();
            let is_forbidden = value
                .get("is_forbidden")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            return Some(Self {
                is_forbidden,
                models,
            });
        }

        // codex shape
        if value.get("hourly_percentage").is_some() || value.get("weekly_percentage").is_some() {
            let mut models = Vec::new();
            if let Some(p) = value.get("hourly_percentage").and_then(|v| v.as_i64()) {
                models.push(ModelQuota {
                    name: "hourly".to_string(),
                    percentage: p as i32,
                });
            }
            if let Some(p) = value.get("weekly_percentage").and_then(|v| v.as_i64()) {
                models.push(ModelQuota {
                    name: "weekly".to_string(),
                    percentage: p as i32,
                });
            }
            return Some(Self {
                is_forbidden: false,
                models,
            });
        }

        // gemini wrapped quota
        if let Some(inner) = value.get("quota") {
            if let Some(models_array) = inner.get("models").and_then(|v| v.as_array()) {
                let models: Vec<ModelQuota> = models_array
                    .iter()
                    .filter_map(|m| {
                        let name = m.get("name")?.as_str()?.to_string();
                        let percentage = m.get("percentage")?.as_i64()? as i32;
                        Some(ModelQuota { name, percentage })
                    })
                    .collect();
                let is_forbidden = inner
                    .get("is_forbidden")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !models.is_empty() {
                    return Some(Self {
                        is_forbidden,
                        models,
                    });
                }
            }
        }

        None
    }

    pub fn average_percentage(&self) -> f64 {
        if self.models.is_empty() {
            return 0.0;
        }
        let sum: i32 = self.models.iter().map(|m| m.percentage).sum();
        sum as f64 / self.models.len() as f64
    }

    pub fn min_percentage(&self) -> i32 {
        self.models
            .iter()
            .map(|m| m.percentage)
            .min()
            .unwrap_or(0)
    }
}
