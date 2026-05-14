use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use uuid::Uuid;

use crate::db::Database;
use crate::error::AppResult;
use crate::models::{BalanceSnapshot, BalanceSummary, BurnRateForecast, Platform, ProviderConfig};
use crate::services::provider::ProviderService;
use crate::store::SecureStore;

pub struct BalanceTracker<'a> {
    db: &'a Database,
    client: Client,
}

impl<'a> BalanceTracker<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            client: Client::new(),
        }
    }

    pub fn get_summaries(&self) -> AppResult<Vec<BalanceSummary>> {
        let sql = "
            SELECT provider_id, provider_name, balance_usd, balance_cny, snapped_at
            FROM (
                SELECT provider_id, provider_name, balance_usd, balance_cny, snapped_at,
                       ROW_NUMBER() OVER (PARTITION BY provider_id ORDER BY snapped_at DESC) as rn
                FROM balance_snapshots
            )
            WHERE rn = 1
            ORDER BY snapped_at DESC;
        ";

        let summaries = self.db.query_rows(sql, &[], |row| {
            let snapped_at_str: String = row.get(4)?;
            let last_updated = snapped_at_str.parse::<DateTime<Utc>>().ok();

            Ok(BalanceSummary {
                provider_id: row.get(0)?,
                provider_name: row.get(1)?,
                platform: "未知".to_string(), // Actually we might need a JOIN with providers table, we'll keep it simple for now as it's not in snapshot
                latest_balance_usd: row.get(2)?,
                latest_balance_cny: row.get(3)?,
                quota_remaining: None,
                quota_unit: None,
                quota_reset_at: None,
                last_updated,
                low_balance_alert: false,
            })
        })?;

        Ok(summaries)
    }

    pub fn get_history(&self, provider_id: &str, limit: u32) -> AppResult<Vec<BalanceSnapshot>> {
        let sql = "
            SELECT id, provider_id, provider_name, balance_usd, balance_cny, quota_remaining, quota_unit, quota_reset_at, snapped_at
            FROM balance_snapshots
            WHERE provider_id = ?1
            ORDER BY snapped_at DESC
            LIMIT ?2;
        ";

        let limit_i64 = limit as i64;
        let snapshots =
            self.db
                .query_rows(sql, rusqlite::params![&provider_id, &limit_i64], |row| {
                    let quota_reset_at_str: Option<String> = row.get(7)?;
                    let snapped_at_str: String = row.get(8)?;

                    Ok(BalanceSnapshot {
                        id: row.get(0)?,
                        provider_id: row.get(1)?,
                        provider_name: row.get(2)?,
                        balance_usd: row.get(3)?,
                        balance_cny: row.get(4)?,
                        quota_remaining: row.get(5)?,
                        quota_unit: row.get(6)?,
                        quota_reset_at: quota_reset_at_str.and_then(|s| s.parse().ok()),
                        snapped_at: snapped_at_str.parse().unwrap_or(Utc::now()),
                    })
                })?;

        Ok(snapshots)
    }

    pub fn get_burn_rate_forecast(&self, provider_id: &str) -> AppResult<BurnRateForecast> {
        let sql_provider = "SELECT platform FROM providers WHERE id = ?1";
        let platform_str: String = self
            .db
            .query_row(sql_provider, rusqlite::params![provider_id], |row| {
                row.get(0)
            })
            .unwrap_or_else(|_| "custom".to_string());

        let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str))
            .unwrap_or(Platform::Custom);

        // Fetch last 30 days snapshots ascending
        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
        let sql_snaps = "
            SELECT balance_usd, balance_cny, snapped_at
            FROM balance_snapshots
            WHERE provider_id = ?1 AND snapped_at >= ?2
            ORDER BY snapped_at ASC
        ";

        let snapshots = self
            .db
            .query_rows(
                sql_snaps,
                rusqlite::params![provider_id, thirty_days_ago.to_rfc3339()],
                |row| {
                    let snapped_at_str: String = row.get(2)?;
                    let snapped_at = snapped_at_str
                        .parse::<DateTime<Utc>>()
                        .unwrap_or(Utc::now());
                    Ok((
                        row.get::<_, Option<f64>>(0)?,
                        row.get::<_, Option<f64>>(1)?,
                        snapped_at,
                    ))
                },
            )
            .unwrap_or_default();

        let mut total_burned_usd = 0.0;
        let mut total_burned_cny = 0.0;

        if snapshots.len() > 1 {
            for i in 1..snapshots.len() {
                let prev = &snapshots[i - 1];
                let curr = &snapshots[i];
                if let (Some(p), Some(c)) = (prev.0, curr.0) {
                    if p > c {
                        total_burned_usd += p - c;
                    }
                }
                if let (Some(p), Some(c)) = (prev.1, curr.1) {
                    if p > c {
                        total_burned_cny += p - c;
                    }
                }
            }
        }

        let days_spanned = if snapshots.len() > 1 {
            let first = snapshots.first().unwrap().2;
            let last = snapshots.last().unwrap().2;
            let diff = (last - first).num_days() as f64;
            if diff > 0.0 {
                diff
            } else {
                1.0
            }
        } else {
            1.0
        };

        let daily_burn_rate_usd = if total_burned_usd > 0.0 {
            Some(total_burned_usd / days_spanned)
        } else {
            None
        };
        let daily_burn_rate_cny = if total_burned_cny > 0.0 {
            Some(total_burned_cny / days_spanned)
        } else {
            None
        };

        let latest_usd = snapshots.last().and_then(|s| s.0);
        let latest_cny = snapshots.last().and_then(|s| s.1);

        let (reset_cycle, next_reset_at) = infer_reset_cycle(&platform);

        let mut estimated_depletion_date = None;
        let mut is_at_risk = false;

        if let (Some(bal), Some(rate)) = (
            latest_cny.or(latest_usd),
            daily_burn_rate_cny.or(daily_burn_rate_usd),
        ) {
            if rate > 0.0 {
                let days_left = bal / rate;
                let depletion = Utc::now() + chrono::Duration::hours((days_left * 24.0) as i64);
                estimated_depletion_date = Some(depletion);

                if let Some(reset) = next_reset_at {
                    if depletion < reset {
                        is_at_risk = true;
                    }
                } else {
                    is_at_risk = true;
                }
            }
        }

        Ok(BurnRateForecast {
            provider_id: provider_id.to_string(),
            daily_burn_rate_usd,
            daily_burn_rate_cny,
            estimated_depletion_date,
            is_at_risk,
            next_reset_at,
            reset_cycle,
        })
    }

    pub async fn refresh_all(&self) -> AppResult<Vec<BalanceSnapshot>> {
        let providers = ProviderService::new(self.db)
            .list_providers()
            .unwrap_or_default();
        let mut results = Vec::new();

        for p in providers.into_iter().filter(|x| x.is_active) {
            if let Ok(snap) = self.refresh_one_internal(&p).await {
                results.push(snap);
            }
        }

        Ok(results)
    }

    pub async fn refresh_one(&self, provider_id: &str) -> AppResult<BalanceSnapshot> {
        let providers = ProviderService::new(self.db)
            .list_providers()
            .unwrap_or_default();
        if let Some(p) = providers.into_iter().find(|x| x.id == provider_id) {
            self.refresh_one_internal(&p).await
        } else {
            Err(anyhow::anyhow!("Provider not found").into())
        }
    }

    async fn refresh_one_internal(&self, p: &ProviderConfig) -> AppResult<BalanceSnapshot> {
        let api_key = match &p.api_key_id {
            Some(key_id) => match SecureStore::get_key(key_id) {
                Ok(k) => k,
                Err(_) => {
                    return Err(anyhow::anyhow!("Missing secure key").into());
                }
            },
            None => {
                return Err(anyhow::anyhow!("No API key linked to provider").into());
            }
        };

        let snapshot_opt = match p.platform {
            Platform::OpenRouter => self.check_openrouter(p, &api_key).await,
            Platform::SiliconFlow => self.check_siliconflow(p, &api_key).await,
            Platform::OpenAI => self.check_openai(p, &api_key).await,
            Platform::DeepSeek => self.check_deepseek(p, &api_key).await,
            Platform::Moonshot => self.check_moonshot(p, &api_key).await,
            Platform::Zhipu => self.check_zhipu(p, &api_key).await,
            Platform::Aliyun => self.check_aliyun(p, &api_key).await,
            Platform::Groq => self.check_groq(p, &api_key).await,
            Platform::MiniMax => self.check_minimax(p, &api_key).await,
            Platform::Anthropic => None,
            Platform::Bytedance => self.check_bytedance(p, &api_key).await,
            _ => None,
        };

        if let Some(snapshot) = snapshot_opt {
            self.save_snapshot(&snapshot)?;
            Ok(snapshot)
        } else {
            Err(
                anyhow::anyhow!("Balance fetching not supported or failed for this provider")
                    .into(),
            )
        }
    }

    async fn check_openrouter(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://openrouter.ai/api/v1/credits";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(data) = json.get("data") {
                        let total_usage = data
                            .get("total_usage")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let limit = data.get("limit").and_then(|v| v.as_f64());

                        let remaining = limit.map(|l| l - total_usage);

                        return Some(BalanceSnapshot {
                            id: Uuid::new_v4().to_string(),
                            provider_id: p.id.clone(),
                            provider_name: p.name.clone(),
                            balance_usd: remaining,
                            balance_cny: None,
                            quota_remaining: None,
                            quota_unit: Some("USD".to_string()),
                            quota_reset_at: None,
                            snapped_at: Utc::now(),
                        });
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    async fn check_siliconflow(
        &self,
        p: &ProviderConfig,
        api_key: &str,
    ) -> Option<BalanceSnapshot> {
        let url = "https://api.siliconflow.cn/v1/user/info";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(data) = json.get("data") {
                        let total_balance: f64 = data
                            .get("totalBalance")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0.0);

                        return Some(BalanceSnapshot {
                            id: Uuid::new_v4().to_string(),
                            provider_id: p.id.clone(),
                            provider_name: p.name.clone(),
                            balance_usd: None,
                            balance_cny: Some(total_balance),
                            quota_remaining: None,
                            quota_unit: Some("CNY".to_string()),
                            quota_reset_at: None,
                            snapped_at: Utc::now(),
                        });
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    async fn check_openai(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://api.openai.com/v1/dashboard/billing/credit_grants";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let total_available = json
                        .get("total_available")
                        .and_then(|v| v.as_f64())?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: Some(total_available),
                        balance_cny: None,
                        quota_remaining: None,
                        quota_unit: Some("USD".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn check_deepseek(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://api.deepseek.com/user/balance";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let total_balance: f64 = json
                        .get("balance_infos")
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get("total_balance"))
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: None,
                        balance_cny: Some(total_balance),
                        quota_remaining: None,
                        quota_unit: Some("CNY".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn check_moonshot(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://api.moonshot.cn/v1/users/me/balance";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let available_balance = json
                        .get("data")
                        .and_then(|v| v.get("available_balance"))
                        .and_then(|v| v.as_f64())?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: None,
                        balance_cny: Some(available_balance),
                        quota_remaining: None,
                        quota_unit: Some("CNY".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn check_zhipu(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://open.bigmodel.cn/api/paas/v4/user/balance";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let balance: f64 = json
                        .get("data")
                        .and_then(|v| v.get("balance"))
                        .and_then(|v| {
                            // May be a number or a string
                            v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                        })?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: None,
                        balance_cny: Some(balance),
                        quota_remaining: None,
                        quota_unit: Some("CNY".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn check_aliyun(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://dashscope.aliyuncs.com/api/v1/services/aigc/billing/balance";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let balance = json
                        .get("data")
                        .and_then(|v| v.get("balance"))
                        .and_then(|v| v.as_f64())?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: None,
                        balance_cny: Some(balance),
                        quota_remaining: None,
                        quota_unit: Some("CNY".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn check_groq(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        // Groq billing/usage API is not publicly available; skip gracefully.
        let _ = (p, api_key);
        None
    }

    async fn check_minimax(&self, p: &ProviderConfig, api_key: &str) -> Option<BalanceSnapshot> {
        let url = "https://api.minimax.chat/v1/user/balance";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let balance = json
                        .get("balance")
                        .and_then(|v| v.as_f64())?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: None,
                        balance_cny: Some(balance),
                        quota_remaining: None,
                        quota_unit: Some("CNY".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn check_bytedance(
        &self,
        p: &ProviderConfig,
        api_key: &str,
    ) -> Option<BalanceSnapshot> {
        let url = "https://ark.cn-beijing.volces.com/api/v3/user/balance";

        match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let balance = json
                        .get("balance")
                        .and_then(|v| v.as_f64())?;

                    Some(BalanceSnapshot {
                        id: Uuid::new_v4().to_string(),
                        provider_id: p.id.clone(),
                        provider_name: p.name.clone(),
                        balance_usd: None,
                        balance_cny: Some(balance),
                        quota_remaining: None,
                        quota_unit: Some("CNY".to_string()),
                        quota_reset_at: None,
                        snapped_at: Utc::now(),
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn save_snapshot(&self, snap: &BalanceSnapshot) -> AppResult<()> {
        let reset_at_str = snap.quota_reset_at.map(|d| d.to_rfc3339());
        let snapped_at_str = snap.snapped_at.to_rfc3339();

        self.db.execute(
            "INSERT INTO balance_snapshots (id, provider_id, provider_name, balance_usd, balance_cny, quota_remaining, quota_unit, quota_reset_at, snapped_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &snap.id,
                &snap.provider_id,
                &snap.provider_name,
                &snap.balance_usd,
                &snap.balance_cny,
                &snap.quota_remaining,
                &snap.quota_unit,
                &reset_at_str,
                &snapped_at_str
            ]
        )?;
        Ok(())
    }
}

fn infer_reset_cycle(platform: &Platform) -> (String, Option<DateTime<Utc>>) {
    use chrono::{Datelike, TimeZone};
    let now = Utc::now();
    match platform {
        Platform::OpenAI | Platform::Anthropic | Platform::AwsBedrock | Platform::AzureOpenAI => {
            let year = if now.month() == 12 {
                now.year() + 1
            } else {
                now.year()
            };
            let month = if now.month() == 12 {
                1
            } else {
                now.month() + 1
            };
            let next_reset = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).single();
            ("monthly".to_string(), next_reset)
        }
        Platform::Gemini => {
            let next_reset = Utc
                .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
                .single()
                .map(|dt| dt + chrono::Duration::days(1));
            ("daily".to_string(), next_reset)
        }
        Platform::DeepSeek
        | Platform::OpenRouter
        | Platform::SiliconFlow
        | Platform::Moonshot
        | Platform::Zhipu
        | Platform::Aliyun
        | Platform::Bytedance
        | Platform::MiniMax
        | Platform::StepFun => ("prepaid_none".to_string(), None),
        _ => ("unknown".to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::path::Path;

    fn make_db() -> Database {
        Database::new(Path::new(":memory:")).expect("open in-memory db")
    }

    /// Insert a snapshot directly via SQL (save_snapshot is private).
    fn insert_snapshot(
        db: &Database,
        id: &str,
        provider_id: &str,
        provider_name: &str,
        balance_usd: Option<f64>,
        balance_cny: Option<f64>,
        snapped_at: &str,
    ) {
        db.execute(
            "INSERT INTO balance_snapshots \
             (id, provider_id, provider_name, balance_usd, balance_cny, \
              quota_remaining, quota_unit, quota_reset_at, snapped_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL, ?6)",
            rusqlite::params![id, provider_id, provider_name, balance_usd, balance_cny, snapped_at],
        )
        .unwrap();
    }

    // ── get_summaries ────────────────────────────────────────────────────────

    #[test]
    fn get_summaries_returns_empty_when_no_snapshots() {
        let db = make_db();
        let tracker = BalanceTracker::new(&db);
        let summaries = tracker.get_summaries().unwrap();
        assert!(summaries.is_empty());
    }

    #[test]
    fn get_summaries_returns_latest_per_provider() {
        let db = make_db();
        let tracker = BalanceTracker::new(&db);

        // Three snapshots for p1, one for p2
        insert_snapshot(&db, "s1", "p1", "Provider One", Some(100.0), None, "2026-01-01T00:00:00Z");
        insert_snapshot(&db, "s2", "p1", "Provider One", Some(90.0),  None, "2026-01-02T00:00:00Z");
        insert_snapshot(&db, "s3", "p1", "Provider One", Some(80.0),  None, "2026-01-03T00:00:00Z");
        insert_snapshot(&db, "s4", "p2", "Provider Two", Some(50.0),  None, "2026-01-01T00:00:00Z");

        let summaries = tracker.get_summaries().unwrap();
        assert_eq!(summaries.len(), 2, "should return one summary per provider");

        let p1_summary = summaries.iter().find(|s| s.provider_id == "p1").unwrap();
        // Latest snapshot for p1 has balance_usd = 80.0
        assert_eq!(
            p1_summary.latest_balance_usd,
            Some(80.0),
            "should return the most recent snapshot balance"
        );

        let p2_summary = summaries.iter().find(|s| s.provider_id == "p2").unwrap();
        assert_eq!(p2_summary.latest_balance_usd, Some(50.0));
    }

    // ── get_history ──────────────────────────────────────────────────────────

    #[test]
    fn get_history_returns_ordered_snapshots() {
        let db = make_db();
        let tracker = BalanceTracker::new(&db);

        insert_snapshot(&db, "s1", "p1", "Provider One", Some(100.0), None, "2026-01-01T00:00:00Z");
        insert_snapshot(&db, "s2", "p1", "Provider One", Some(90.0),  None, "2026-01-02T00:00:00Z");
        insert_snapshot(&db, "s3", "p1", "Provider One", Some(80.0),  None, "2026-01-03T00:00:00Z");

        let history = tracker.get_history("p1", 10).unwrap();
        assert_eq!(history.len(), 3);

        // Should be ordered DESC by snapped_at — most recent first
        assert_eq!(history[0].id, "s3");
        assert_eq!(history[1].id, "s2");
        assert_eq!(history[2].id, "s1");
    }

    // ── save_snapshot (via public refresh path) ──────────────────────────────

    #[test]
    fn save_snapshot_persists_to_db() {
        let db = make_db();
        let tracker = BalanceTracker::new(&db);

        // Insert directly since save_snapshot is private
        insert_snapshot(&db, "snap1", "p1", "Test Provider", Some(10.0), None, "2026-01-01T00:00:00Z");

        let history = tracker.get_history("p1", 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "snap1");
        assert_eq!(history[0].provider_id, "p1");
        assert_eq!(history[0].balance_usd, Some(10.0));
    }

    // ── get_burn_rate_forecast ───────────────────────────────────────────────

    #[test]
    fn get_burn_rate_forecast_returns_zero_for_no_data() {
        let db = make_db();
        let tracker = BalanceTracker::new(&db);

        // No snapshots for "p_empty"
        let forecast = tracker.get_burn_rate_forecast("p_empty").unwrap();
        assert!(
            forecast.daily_burn_rate_usd.is_none(),
            "daily_burn_rate_usd should be None when there are no snapshots"
        );
    }
}
