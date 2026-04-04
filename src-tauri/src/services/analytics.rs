use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenTrend {
    pub date: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdeAccountStatusData {
    pub name: String, // 'Active', 'Forbidden', 'RateLimited', 'Expired'
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopConsumer {
    pub client_app: String,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub active_ide_accounts: u64,
    pub total_user_tokens: u64,
    pub today_total_tokens: u64,
    pub forbidden_accounts_ratio: f64,
    pub token_trends: Vec<TokenTrend>,
    pub ide_status_distribution: Vec<IdeAccountStatusData>,
    pub top_consumers: Vec<TopConsumer>,
}

pub struct AnalyticsService;

impl AnalyticsService {
    pub fn get_metrics(db: &Database, days: u32) -> crate::error::AppResult<DashboardMetrics> {
        let active_ide_accounts: u64 = db.query_row(
            "SELECT count(*) FROM ide_accounts WHERE status = 'active'",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);

        let total_ide_accounts: u64 = db.query_row(
            "SELECT count(*) FROM ide_accounts",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);

        let forbidden_ide_accounts: u64 = db.query_row(
            "SELECT count(*) FROM ide_accounts WHERE status = 'forbidden' OR status = 'rate_limited'",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);

        let forbidden_accounts_ratio = if total_ide_accounts > 0 {
            (forbidden_ide_accounts as f64) / (total_ide_accounts as f64)
        } else {
            0.0
        };

        let total_user_tokens: u64 = db.query_row(
            "SELECT count(*) FROM user_tokens",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);

        let today_total_tokens: u64 = db.query_row(
            "SELECT ifnull(sum(total_tokens), 0) FROM token_usage_records WHERE date(created_at) = date('now')",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);

        let limit = days;
        let trend_query = format!(
            "SELECT date(created_at) as dt, ifnull(sum(prompt_tokens), 0), ifnull(sum(completion_tokens), 0), ifnull(sum(total_tokens), 0)
             FROM token_usage_records
             WHERE created_at >= date('now', '-{} days')
             GROUP BY dt
             ORDER BY dt ASC", limit
        );
        let token_trends = db.query_rows(&trend_query, &[], |row| {
            Ok(TokenTrend {
                date: row.get(0)?,
                prompt_tokens: row.get(1)?,
                completion_tokens: row.get(2)?,
                total_tokens: row.get(3)?,
            })
        }).unwrap_or_default();

        let mut ide_status_distribution = vec![
            IdeAccountStatusData { name: "Active".to_string(), value: active_ide_accounts },
            IdeAccountStatusData { name: "Forbidden".to_string(), value: forbidden_ide_accounts },
        ];
        
        let expired: u64 = db.query_row(
            "SELECT count(*) FROM ide_accounts WHERE status = 'expired'",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);
        ide_status_distribution.push(IdeAccountStatusData { name: "Expired".to_string(), value: expired });

        let limit_consumers = 10;
        let top_consumers_query = format!(
            "SELECT client_app, ifnull(sum(total_tokens),0) as tokens
             FROM token_usage_records
             GROUP BY client_app
             ORDER BY tokens DESC
             LIMIT {}", limit_consumers
        );
        let top_consumers = db.query_rows(&top_consumers_query, &[], |row| {
            Ok(TopConsumer {
                client_app: row.get(0)?,
                total_tokens: row.get(1)?,
            })
        }).unwrap_or_default();

        Ok(DashboardMetrics {
            active_ide_accounts,
            total_user_tokens,
            today_total_tokens,
            forbidden_accounts_ratio,
            token_trends,
            ide_status_distribution,
            top_consumers,
        })
    }
}
