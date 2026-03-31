use crate::{models::Balance, AppError};
use chrono::Utc;
use tauri::State;
use crate::db::Database;
use crate::store::SecureStore;
use crate::models::Platform;

/// 获取所有账号余额（从缓存）
#[tauri::command]
pub async fn get_all_balances(db: State<'_, Database>) -> Result<Vec<Balance>, AppError> {
    type Row = (String, String, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<String>, Option<String>);
    let rows: Vec<Row> = db.query_rows(
        "SELECT k.id, k.platform, b.balance_usd, b.balance_cny, b.total_usage_usd,
                b.quota_remaining, b.quota_reset_at, b.synced_at
         FROM api_keys k
         LEFT JOIN balances b ON k.id = b.key_id
         ORDER BY k.created_at DESC",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
    )?;

    Ok(rows.into_iter().map(|(key_id, platform_str, balance_usd, balance_cny, total_usage_usd, quota_remaining, quota_reset_at, synced_at)| {
        Balance {
            key_id,
            platform: serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom),
            balance_usd, balance_cny, total_usage_usd, quota_remaining,
            quota_reset_at: quota_reset_at.and_then(|s| s.parse().ok()),
            synced_at: synced_at.and_then(|s| s.parse().ok()).unwrap_or_else(Utc::now),
        }
    }).collect())
}

/// 刷新指定 Key 的余额（调用平台 API）
#[tauri::command]
pub async fn get_platform_balance(
    db: State<'_, Database>,
    key_id: String,
) -> Result<Balance, AppError> {
    let secret = SecureStore::get_key(&key_id)?;

    let (platform_str, _): (String, Option<String>) = db.query_one(
        "SELECT platform, base_url FROM api_keys WHERE id = ?1",
        &[&key_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str))
        .unwrap_or(Platform::Custom);

    let mut balance = crate::services::balance::fetch_balance(&platform, &secret).await?;
    balance.key_id = key_id.clone();

    db.execute(
        "INSERT OR REPLACE INTO balances
         (key_id, platform, balance_usd, balance_cny, total_usage_usd, quota_remaining, quota_reset_at, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        &[
            &key_id,
            &platform_str,
            &balance.balance_usd as &dyn rusqlite::ToSql,
            &balance.balance_cny as &dyn rusqlite::ToSql,
            &balance.total_usage_usd as &dyn rusqlite::ToSql,
            &balance.quota_remaining as &dyn rusqlite::ToSql,
            &balance.quota_reset_at.as_ref().map(|t| t.to_rfc3339()) as &dyn rusqlite::ToSql,
            &balance.synced_at.to_rfc3339(),
        ],
    )?;

    Ok(balance)
}

/// 刷新所有有效 Key 的余额
#[tauri::command]
pub async fn refresh_all_balances(db: State<'_, Database>) -> Result<Vec<Balance>, AppError> {
    let key_ids: Vec<String> = db.query_rows(
        "SELECT id FROM api_keys WHERE status = 'valid'",
        &[],
        |r| r.get(0),
    )?;

    let mut results = Vec::new();
    for id in key_ids {
        match get_platform_balance(db.clone(), id).await {
            Ok(b) => results.push(b),
            Err(_) => {}
        }
    }

    Ok(results)
}
