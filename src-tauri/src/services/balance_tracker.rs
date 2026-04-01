/// 余额追踪器服务
///
/// 按需或定期从各 Provider 的配置获取 API Key，调用对应平台余额 API，
/// 将快照写入 balance_snapshots 表。
///
/// 设计原则：
/// - 主动查询（轮询），不依赖代理拦截
/// - 每次查询写一条快照（时序），保留 30 天历史
/// - 不支持余额 API 的平台写入 None 但仍记录查询时间

use chrono::Utc;
use reqwest::Client;
use rusqlite::params;
use serde_json::Value;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{BalanceSummary, BalanceSnapshot, Platform};
use crate::services::provider::ProviderService;
use crate::store::SecureStore;

pub struct BalanceTracker<'a> {
    db: &'a Database,
}

impl<'a> BalanceTracker<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 公共 API
    // ─────────────────────────────────────────────────────────────────────────

    /// 刷新所有 Provider 的余额（全量）
    pub async fn refresh_all(&self) -> AppResult<Vec<BalanceSnapshot>> {
        let providers = ProviderService::new(self.db).list_providers()?;
        let mut snapshots = Vec::new();

        for p in &providers {
            let Some(ref key_id) = p.api_key_id else {
                continue;
            };

            let secret = match SecureStore::get_key(key_id) {
                Ok(s) => s,
                Err(e) => {
                    warn!("获取 Provider {} 的 Key 失败: {}", p.name, e);
                    continue;
                }
            };

            let base_url = p.base_url.clone();
            match self.fetch_provider_balance(&p.id, &p.name, &p.platform, &secret, base_url.as_deref()).await {
                Ok(snap) => {
                    self.save_snapshot(&snap)?;
                    snapshots.push(snap);
                }
                Err(e) => {
                    warn!("查询 Provider {} 余额失败: {}", p.name, e);
                }
            }
        }

        // 清理 30 天前的历史快照
        self.cleanup_old_snapshots(30)?;

        Ok(snapshots)
    }

    /// 刷新单个 Provider 余额
    pub async fn refresh_one(&self, provider_id: &str) -> AppResult<BalanceSnapshot> {
        let providers = ProviderService::new(self.db).list_providers()?;
        let p = providers.iter()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| AppError::Message(format!("Provider {} 不存在", provider_id)))?;

        let key_id = p.api_key_id.as_ref()
            .ok_or_else(|| AppError::Message("此 Provider 未绑定 API Key".into()))?;
        let secret = SecureStore::get_key(key_id)?;

        let snap = self.fetch_provider_balance(&p.id, &p.name, &p.platform, &secret, p.base_url.as_deref()).await?;
        self.save_snapshot(&snap)?;
        Ok(snap)
    }

    /// 获取所有 Provider 的最新余额汇总
    pub fn get_summaries(&self) -> AppResult<Vec<BalanceSummary>> {
        let rows = self.db.query_rows(
            "SELECT
                bs.provider_id,
                bs.provider_name,
                bs.balance_usd,
                bs.balance_cny,
                bs.quota_remaining,
                bs.quota_unit,
                bs.quota_reset_at,
                bs.snapped_at,
                p.platform,
                p.notes
             FROM balance_snapshots bs
             LEFT JOIN providers p ON bs.provider_id = p.id
             WHERE bs.snapped_at = (
                 SELECT MAX(s2.snapped_at) FROM balance_snapshots s2
                 WHERE s2.provider_id = bs.provider_id
             )
             ORDER BY bs.snapped_at DESC",
            &[],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,   // provider_id
                    row.get::<_, String>(1)?,   // provider_name
                    row.get::<_, Option<f64>>(2)?,  // balance_usd
                    row.get::<_, Option<f64>>(3)?,  // balance_cny
                    row.get::<_, Option<f64>>(4)?,  // quota_remaining
                    row.get::<_, Option<String>>(5)?, // quota_unit
                    row.get::<_, Option<String>>(6)?, // quota_reset_at
                    row.get::<_, String>(7)?,   // snapped_at
                    row.get::<_, Option<String>>(8)?, // platform
                    row.get::<_, Option<String>>(9)?, // notes (low balance threshold)
                ))
            },
        )?;

        let summaries = rows.into_iter().map(|(provider_id, provider_name, balance_usd, balance_cny, quota_remaining, quota_unit, quota_reset_at, snapped_at, platform, _notes)| {
            // 低余额告警：USD < 1 或 CNY < 7
            let low_balance_alert = balance_usd.map(|b| b < 1.0).unwrap_or(false)
                || balance_cny.map(|b| b < 7.0).unwrap_or(false);

            BalanceSummary {
                provider_id,
                provider_name,
                platform: platform.unwrap_or_else(|| "custom".into()),
                latest_balance_usd: balance_usd,
                latest_balance_cny: balance_cny,
                quota_remaining,
                quota_unit,
                quota_reset_at: quota_reset_at.and_then(|s| s.parse().ok()),
                last_updated: snapped_at.parse().ok(),
                low_balance_alert,
            }
        }).collect();

        Ok(summaries)
    }

    /// 获取某 Provider 的历史余额趋势（最近 N 条快照）
    pub fn get_history(&self, provider_id: &str, limit: u32) -> AppResult<Vec<BalanceSnapshot>> {
        self.db.query_rows(
            "SELECT id, provider_id, provider_name, balance_usd, balance_cny,
                    quota_remaining, quota_unit, quota_reset_at, snapped_at
             FROM balance_snapshots
             WHERE provider_id = ?1
             ORDER BY snapped_at DESC
             LIMIT ?2",
            &[&provider_id, &(limit as i64)],
            |row| {
                Ok(BalanceSnapshot {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    provider_name: row.get(2)?,
                    balance_usd: row.get(3)?,
                    balance_cny: row.get(4)?,
                    quota_remaining: row.get(5)?,
                    quota_unit: row.get(6)?,
                    quota_reset_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
                    snapped_at: row.get::<_, String>(8)?.parse().unwrap_or_else(|_| Utc::now()),
                })
            },
        ).map_err(Into::into)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 内部：各平台余额查询
    // ─────────────────────────────────────────────────────────────────────────

    async fn fetch_provider_balance(
        &self,
        provider_id: &str,
        provider_name: &str,
        platform: &Platform,
        secret: &str,
        base_url: Option<&str>,
    ) -> AppResult<BalanceSnapshot> {
        info!("查询 Provider {} 余额", provider_name);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()?;

        let (balance_usd, balance_cny, quota_remaining, quota_unit) = match platform {
            Platform::OpenAI        => fetch_openai(&client, secret).await,
            Platform::DeepSeek      => fetch_deepseek(&client, secret).await,
            Platform::Moonshot      => fetch_moonshot(&client, secret).await,
            Platform::OpenRouter    => fetch_openrouter(&client, secret).await,
            Platform::SiliconFlow   => fetch_siliconflow(&client, secret).await,
            Platform::Zhipu         => fetch_zhipu(&client, secret).await,
            Platform::MiniMax       => fetch_minimax(&client, secret).await,
            // 通用 OpenAI 兼容格式：如果有自定义 base_url 尝试通用用量接口
            Platform::Custom if base_url.is_some() => {
                fetch_generic_openai_compat(&client, secret, base_url.unwrap()).await
            }
            // 不支持余额 API 的平台
            _ => (None, None, None, None),
        };

        Ok(BalanceSnapshot {
            id: Uuid::new_v4().to_string(),
            provider_id: provider_id.to_string(),
            provider_name: provider_name.to_string(),
            balance_usd,
            balance_cny,
            quota_remaining,
            quota_unit,
            quota_reset_at: None,
            snapped_at: Utc::now(),
        })
    }

    fn save_snapshot(&self, snap: &BalanceSnapshot) -> AppResult<()> {
        self.db.execute(
            "INSERT INTO balance_snapshots
             (id, provider_id, provider_name, balance_usd, balance_cny, quota_remaining, quota_unit, quota_reset_at, snapped_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                snap.id,
                snap.provider_id,
                snap.provider_name,
                snap.balance_usd,
                snap.balance_cny,
                snap.quota_remaining,
                snap.quota_unit,
                snap.quota_reset_at.as_ref().map(|t| t.to_rfc3339()),
                snap.snapped_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn cleanup_old_snapshots(&self, days: i64) -> AppResult<()> {
        self.db.execute(
            "DELETE FROM balance_snapshots WHERE snapped_at < datetime('now', ?1)",
            &[&format!("-{} days", days)],
        )?;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 各平台余额查询实现（返回 (usd, cny, quota_remaining, quota_unit)）
// ─────────────────────────────────────────────────────────────────────────────

type BalanceTuple = (Option<f64>, Option<f64>, Option<f64>, Option<String>);

async fn fetch_openai(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://api.openai.com/dashboard/billing/credit_grants")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            let remaining = json["total_available"].as_f64()
                .or_else(|| {
                    let granted = json["total_granted"].as_f64().unwrap_or(0.0);
                    let used    = json["total_used"].as_f64().unwrap_or(0.0);
                    Some(granted - used)
                });
            (remaining, None, remaining, Some("USD".into()))
        }
        _ => (None, None, None, None),
    }
}

async fn fetch_deepseek(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://api.deepseek.com/user/balance")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            let infos = json["balance_infos"].as_array().cloned().unwrap_or_default();
            let cny = infos.iter()
                .find(|i| i["currency"].as_str() == Some("CNY"))
                .and_then(|i| i["total_balance"].as_str().and_then(|s| s.parse::<f64>().ok()));
            let usd = infos.iter()
                .find(|i| i["currency"].as_str() == Some("USD"))
                .and_then(|i| i["total_balance"].as_str().and_then(|s| s.parse::<f64>().ok()));
            let unit = if usd.is_some() { "USD" } else { "CNY" };
            (usd, cny, usd.or(cny), Some(unit.into()))
        }
        _ => (None, None, None, None),
    }
}

async fn fetch_moonshot(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://api.moonshot.cn/v1/users/me/balance")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            let available = json["data"]["available_balance"].as_f64();
            (None, available, available, Some("CNY".into()))
        }
        _ => (None, None, None, None),
    }
}

async fn fetch_openrouter(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://openrouter.ai/api/v1/credits")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            // {"data": {"total_credits": "10.000000", "total_usage": "0.123456"}}
            let total = json["data"]["total_credits"].as_str().and_then(|s| s.parse::<f64>().ok());
            let used  = json["data"]["total_usage"].as_str().and_then(|s| s.parse::<f64>().ok());
            let remaining = total.zip(used).map(|(t, u)| t - u).or(total);
            (remaining, None, remaining, Some("USD".into()))
        }
        _ => (None, None, None, None),
    }
}

async fn fetch_siliconflow(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://api.siliconflow.cn/v1/user/info")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            // {"data": {"balance": "100.00", "chargeBalance": "50.00"}}
            let balance = json["data"]["balance"].as_str().and_then(|s| s.parse::<f64>().ok())
                .or_else(|| json["data"]["totalBalance"].as_f64());
            (None, balance, balance, Some("CNY".into()))
        }
        _ => (None, None, None, None),
    }
}

async fn fetch_zhipu(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://open.bigmodel.cn/api/paas/v4/user/billing_account")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            let balance = json["data"]["balance"].as_f64()
                .or_else(|| json["data"]["availableBalance"].as_f64());
            (None, balance, balance, Some("CNY".into()))
        }
        _ => (None, None, None, None),
    }
}

async fn fetch_minimax(client: &Client, secret: &str) -> BalanceTuple {
    let resp = client
        .get("https://api.minimaxi.com/v1/wallet/balance")
        .bearer_auth(secret)
        .send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or_default();
            let balance = json["balance"].as_f64()
                .or_else(|| json["data"]["balance"].as_f64());
            (None, balance, balance, Some("CNY".into()))
        }
        _ => (None, None, None, None),
    }
}

/// 通用 OpenAI 兼容格式：尝试 /v1/dashboard/billing/credit_grants 或 /v1/user/info
async fn fetch_generic_openai_compat(client: &Client, secret: &str, base_url: &str) -> BalanceTuple {
    let base = base_url.trim_end_matches('/');
    // 尝试 NewAPI / OneAPI 风格的用量接口
    let urls = [
        format!("{}/api/user/info", base),
        format!("{}/dashboard/billing/credit_grants", base),
    ];

    for url in &urls {
        if let Ok(resp) = client.get(url).bearer_auth(secret).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    // OneAPI / NewAPI 风格: {"data": {"quota": 500000, "used_quota": 12345}}
                    let quota   = json["data"]["quota"].as_f64();
                    let used    = json["data"]["used_quota"].as_f64();
                    if quota.is_some() {
                        let remaining = quota.zip(used).map(|(q, u)| (q - u) / 500000.0).or(quota.map(|q| q / 500000.0));
                        return (remaining, None, remaining, Some("USD".into()));
                    }
                }
            }
        }
    }

    (None, None, None, None)
}
