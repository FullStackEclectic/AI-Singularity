use crate::{models::Balance, AppError};
use chrono::Utc;
use tauri::State;
use crate::db::Database;
use crate::store::SecureStore;
use crate::models::Platform;

/// 获取所有账号余额（从缓存）
#[tauri::command]
pub async fn get_all_balances(db: State<'_, Database>) -> Result<Vec<Balance>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT k.id, k.platform, b.balance_usd, b.balance_cny, b.total_usage_usd,
                b.quota_remaining, b.quota_reset_at, b.synced_at
         FROM api_keys k
         LEFT JOIN balances b ON k.id = b.key_id
         ORDER BY k.created_at DESC",
    )?;

    let balances = stmt.query_map([], |row| {
        let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", row.get::<_, String>(1)?))
            .unwrap_or(Platform::Custom);
        Ok(Balance {
            key_id: row.get(0)?,
            platform,
            balance_usd: row.get(2)?,
            balance_cny: row.get(3)?,
            total_usage_usd: row.get(4)?,
            quota_remaining: row.get(5)?,
            quota_reset_at: row.get::<_, Option<String>>(6)?.and_then(|s| s.parse().ok()),
            synced_at: row.get::<_, Option<String>>(7)?
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(Utc::now),
        })
    })?
    .filter_map(|r| r.ok())
    .collect();

    Ok(balances)
}

/// 刷新指定 Key 的余额（调用平台 API）
#[tauri::command]
pub async fn get_platform_balance(
    db: State<'_, Database>,
    key_id: String,
) -> Result<Balance, AppError> {
    let secret = SecureStore::get_key(&key_id)?;

    let (platform_str, _base_url) = {
        let conn = db.conn();
        conn.query_row(
            "SELECT platform, base_url FROM api_keys WHERE id = ?1",
            rusqlite::params![key_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )?
    };

    let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str))
        .unwrap_or(Platform::Custom);

    let balance = crate::services::balance::fetch_balance(&platform, &secret).await?;

    // 更新到 SQLite 缓存
    {
        let conn = db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO balances
             (key_id, platform, balance_usd, balance_cny, total_usage_usd, quota_remaining, quota_reset_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                key_id,
                platform_str,
                balance.balance_usd,
                balance.balance_cny,
                balance.total_usage_usd,
                balance.quota_remaining,
                balance.quota_reset_at.as_ref().map(|t| t.to_rfc3339()),
                balance.synced_at.to_rfc3339(),
            ],
        )?;
    }

    Ok(balance)
}

/// 刷新所有 Key 的余额
#[tauri::command]
pub async fn refresh_all_balances(db: State<'_, Database>) -> Result<Vec<Balance>, AppError> {
    let key_ids: Vec<String> = {
        let conn = db.conn();
        let mut stmt = conn.prepare("SELECT id FROM api_keys WHERE status = 'valid'")?;
        stmt.query_map([], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect()
    };

    let mut results = Vec::new();
    for id in key_ids {
        // 逐个刷新，忽略单个失败
        match get_platform_balance(db.clone(), id).await {
            Ok(b) => results.push(b),
            Err(_) => {} // 静默跳过失败的 Key
        }
    }

    Ok(results)
}
