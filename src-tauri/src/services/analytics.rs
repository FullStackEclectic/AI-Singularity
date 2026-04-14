use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenTrend {
    pub date: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
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
pub struct ModelCostStats {
    pub model_name: String,
    pub total_cost_usd: f64,
    pub total_requests: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformCostStats {
    pub platform: String,
    pub total_cost_usd: f64,
    pub total_requests: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub active_ide_accounts: u64,
    pub total_user_tokens: u64,
    pub today_total_tokens: u64,
    pub total_cost_today_usd: f64, // NEW
    pub forbidden_accounts_ratio: f64,
    pub token_trends: Vec<TokenTrend>,
    pub ide_status_distribution: Vec<IdeAccountStatusData>,
    pub top_consumers: Vec<TopConsumer>,
    pub model_costs: Vec<ModelCostStats>,       // NEW
    pub platform_costs: Vec<PlatformCostStats>, // NEW
}

pub struct AnalyticsService;

impl AnalyticsService {
    pub fn get_metrics(db: &Database, days: u32) -> crate::error::AppResult<DashboardMetrics> {
        let active_ide_accounts: u64 = db
            .query_row(
                "SELECT count(*) FROM ide_accounts WHERE status = 'active'",
                &[],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_ide_accounts: u64 = db
            .query_row("SELECT count(*) FROM ide_accounts", &[], |row| row.get(0))
            .unwrap_or(0);

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

        let total_user_tokens: u64 = db
            .query_row("SELECT count(*) FROM user_tokens", &[], |row| row.get(0))
            .unwrap_or(0);

        let today_total_tokens: u64 = db.query_row(
            "SELECT ifnull(sum(total_tokens), 0) FROM token_usage_records WHERE date(created_at) = date('now', 'localtime')",
            &[],
            |row| row.get(0)
        ).unwrap_or(0);

        // Calculate Cost Today
        let total_cost_today_usd: f64 = db.query_row(
            "SELECT ifnull(sum(total_cost_usd), 0.0) FROM token_usage_records WHERE date(created_at) = date('now', 'localtime')",
            &[],
            |row| row.get(0)
        ).unwrap_or(0.0);

        let limit = days;
        let trend_query = format!(
            "SELECT date(created_at) as dt, ifnull(sum(prompt_tokens), 0), ifnull(sum(completion_tokens), 0), ifnull(sum(total_tokens), 0), ifnull(sum(total_cost_usd), 0.0)
             FROM token_usage_records
             WHERE created_at >= date('now', '-{} days')
             GROUP BY dt
             ORDER BY dt ASC", limit
        );
        let token_trends = db
            .query_rows(&trend_query, &[], |row| {
                Ok(TokenTrend {
                    date: row.get(0)?,
                    prompt_tokens: row.get(1)?,
                    completion_tokens: row.get(2)?,
                    total_tokens: row.get(3)?,
                    total_cost_usd: row.get(4)?,
                })
            })
            .unwrap_or_default();

        let mut ide_status_distribution = vec![
            IdeAccountStatusData {
                name: "Active".to_string(),
                value: active_ide_accounts,
            },
            IdeAccountStatusData {
                name: "Forbidden".to_string(),
                value: forbidden_ide_accounts,
            },
        ];

        let expired: u64 = db
            .query_row(
                "SELECT count(*) FROM ide_accounts WHERE status = 'expired'",
                &[],
                |row| row.get(0),
            )
            .unwrap_or(0);
        ide_status_distribution.push(IdeAccountStatusData {
            name: "Expired".to_string(),
            value: expired,
        });

        let limit_consumers = 10;
        let top_consumers_query = format!(
            "SELECT client_app, ifnull(sum(total_tokens),0) as tokens
             FROM token_usage_records
             GROUP BY client_app
             ORDER BY tokens DESC
             LIMIT {}",
            limit_consumers
        );
        let top_consumers = db
            .query_rows(&top_consumers_query, &[], |row| {
                Ok(TopConsumer {
                    client_app: row.get(0)?,
                    total_tokens: row.get(1)?,
                })
            })
            .unwrap_or_default();

        // Model Costs (Top 10)
        let model_costs_query = "
            SELECT model_name, ifnull(sum(total_cost_usd), 0.0) as cost, count(id)
            FROM token_usage_records
            GROUP BY model_name
            ORDER BY cost DESC
            LIMIT 10
        ";
        let model_costs = db
            .query_rows(model_costs_query, &[], |row| {
                Ok(ModelCostStats {
                    model_name: row.get(0)?,
                    total_cost_usd: row.get(1)?,
                    total_requests: row.get(2)?,
                })
            })
            .unwrap_or_default();

        // Platform/Provider Costs
        let platform_costs_query = "
            SELECT platform, ifnull(sum(total_cost_usd), 0.0) as cost, count(id)
            FROM token_usage_records
            GROUP BY platform
            ORDER BY cost DESC
        ";
        let platform_costs = db
            .query_rows(platform_costs_query, &[], |row| {
                Ok(PlatformCostStats {
                    platform: row.get(0)?,
                    total_cost_usd: row.get(1)?,
                    total_requests: row.get(2)?,
                })
            })
            .unwrap_or_default();

        Ok(DashboardMetrics {
            active_ide_accounts,
            total_user_tokens,
            today_total_tokens,
            total_cost_today_usd,
            forbidden_accounts_ratio,
            token_trends,
            ide_status_distribution,
            top_consumers,
            model_costs,
            platform_costs,
        })
    }

    #[allow(dead_code)]
    pub fn log_token_usage(
        db: &Database,
        key_id: &str,
        platform: &str,
        model_name: &str,
        client_app: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
        total_tokens: i64,
    ) -> crate::error::AppResult<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        let total_cost_usd = 0.0; // Needs pricing formula, using 0 for now

        db.execute(
            "INSERT INTO token_usage_records (
                id, key_id, platform, model_name, client_app, prompt_tokens, completion_tokens, total_tokens, created_at, total_cost_usd
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![id, key_id, platform, model_name, client_app, prompt_tokens, completion_tokens, total_tokens, created_at, total_cost_usd]
        )?;
        Ok(())
    }
}
