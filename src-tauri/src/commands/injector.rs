use crate::error::AppResult;
use crate::services::ide_injector::IdeInjector;
use crate::models::IdeAccount;
use crate::db::Database;
use tauri::State;

#[tauri::command]
pub async fn force_inject_ide(account_id: String, db: State<'_, Database>) -> AppResult<()> {
    // Read the full account details from DB to get the tokens and profiles
    let mut target_account: Option<IdeAccount> = None;
    
    // We instantiate a pool manager or read directly, since IdeAccount is managed by commands
    // We do a direct SQLite query just for completeness here
    db.query_row(
        "SELECT id, origin_platform, email, token_json, device_profile_json, status, created_at, updated_at FROM ide_accounts WHERE id = ?1",
        rusqlite::params![account_id],
        |row| {
            let token_json: String = row.get(3)?;
            let profile_json: String = row.get(4)?;
            let status_str: String = row.get(5)?;
            let status = match status_str.as_str() {
                "active" => crate::models::AccountStatus::Active,
                "expired" => crate::models::AccountStatus::Expired,
                "forbidden" => crate::models::AccountStatus::Forbidden,
                "rate_limited" => crate::models::AccountStatus::RateLimited,
                _ => crate::models::AccountStatus::Unknown,
            };
            
            let created_at_str: String = row.get(6)?;
            let updated_at_str: String = row.get(7)?;

            Ok(IdeAccount {
                id: row.get(0)?,
                origin_platform: row.get(1)?,
                email: row.get(2)?,
                token: serde_json::from_str(&token_json).unwrap_or_else(|_| crate::models::OAuthToken {
                   access_token: "".into(), refresh_token: "".into(), expires_in: 0, token_type: "".into(), 
                   updated_at: chrono::Utc::now() 
                }),
                status,
                disabled_reason: None,
                is_proxy_disabled: false,
                created_at: created_at_str.parse().unwrap_or(chrono::Utc::now()),
                updated_at: updated_at_str.parse().unwrap_or(chrono::Utc::now()),
                last_used: chrono::Utc::now(),
                device_profile: serde_json::from_str(&profile_json).unwrap_or(None),
                quota_json: None,
            })
        }
    ).map(|a| {
        target_account = Some(a);
    })?;

    if let Some(acc) = target_account {
        IdeInjector::execute_injection(&acc)?;
        Ok(())
    } else {
        Err(crate::error::AppError::Other(anyhow::anyhow!("未能在兵工厂中找到该账户！")))
    }
}
