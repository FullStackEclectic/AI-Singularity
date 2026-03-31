use crate::{
    db::Database,
    models::{ApiKey, KeyStatus, Platform},
    store::SecureStore,
    AppError, AppResult,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AddKeyRequest {
    pub name: String,
    pub platform: Platform,
    pub secret: String,
    pub base_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyRequest {
    pub id: String,
    pub name: Option<String>,
    pub secret: Option<String>,
    pub base_url: Option<String>,
    pub notes: Option<String>,
}

/// 获取所有 Key 列表（不含 secret）
#[tauri::command]
pub async fn list_keys(db: State<'_, Database>) -> Result<Vec<ApiKey>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, platform, base_url, key_preview, status, notes, created_at, last_checked_at
         FROM api_keys ORDER BY created_at DESC",
    )?;

    let keys = stmt
        .query_map([], |row| {
            Ok(ApiKey {
                id: row.get(0)?,
                name: row.get(1)?,
                platform: serde_json::from_str::<Platform>(&format!(
                    "\"{}\"",
                    row.get::<_, String>(2)?
                ))
                .unwrap_or(Platform::Custom),
                base_url: row.get(3)?,
                key_preview: row.get(4)?,
                status: serde_json::from_str::<KeyStatus>(&format!(
                    "\"{}\"",
                    row.get::<_, String>(5)?
                ))
                .unwrap_or(KeyStatus::Unknown),
                notes: row.get(6)?,
                created_at: row.get::<_, String>(7)?.parse().unwrap_or(Utc::now()),
                last_checked_at: row
                    .get::<_, Option<String>>(8)?
                    .and_then(|s| s.parse().ok()),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(keys)
}

/// 添加新 Key
#[tauri::command]
pub async fn add_key(db: State<'_, Database>, request: AddKeyRequest) -> Result<ApiKey, AppError> {
    let id = Uuid::new_v4().to_string();
    let preview = SecureStore::key_preview(&request.secret);
    let now = Utc::now();

    // 存入系统 Keychain
    SecureStore::store_key(&id, &request.secret)?;

    let platform_str = serde_json::to_string(&request.platform)
        .unwrap_or("\"custom\"".to_string())
        .trim_matches('"')
        .to_string();

    // 元数据存入 SQLite
    let conn = db.conn();
    conn.execute(
        "INSERT INTO api_keys (id, name, platform, base_url, key_hash, key_preview, status, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unknown', ?7, ?8)",
        rusqlite::params![
            id,
            request.name,
            platform_str,
            request.base_url,
            "placeholder", // TODO: SHA-256 哈希
            preview,
            request.notes,
            now.to_rfc3339(),
        ],
    )?;

    Ok(ApiKey {
        id,
        name: request.name,
        platform: request.platform,
        base_url: request.base_url,
        key_preview: preview,
        status: KeyStatus::Unknown,
        notes: request.notes,
        created_at: now,
        last_checked_at: None,
    })
}

/// 删除 Key
#[tauri::command]
pub async fn delete_key(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    SecureStore::delete_key(&id).ok(); // Keychain 删除（忽略错误）
    let conn = db.conn();
    conn.execute("DELETE FROM api_keys WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

/// 更新 Key 元数据
#[tauri::command]
pub async fn update_key(
    db: State<'_, Database>,
    request: UpdateKeyRequest,
) -> Result<(), AppError> {
    let conn = db.conn();

    if let Some(secret) = &request.secret {
        SecureStore::store_key(&request.id, secret)?;
        let preview = SecureStore::key_preview(secret);
        conn.execute(
            "UPDATE api_keys SET key_preview = ?1 WHERE id = ?2",
            rusqlite::params![preview, request.id],
        )?;
    }

    if let Some(name) = &request.name {
        conn.execute(
            "UPDATE api_keys SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, request.id],
        )?;
    }

    if let Some(notes) = &request.notes {
        conn.execute(
            "UPDATE api_keys SET notes = ?1 WHERE id = ?2",
            rusqlite::params![notes, request.id],
        )?;
    }

    Ok(())
}

/// 检测 Key 有效性（调用平台 API）
#[tauri::command]
pub async fn check_key(db: State<'_, Database>, id: String) -> Result<KeyStatus, AppError> {
    let secret = SecureStore::get_key(&id)?;

    // 获取 platform 信息
    let (platform, base_url) = {
        let conn = db.conn();
        let result: (String, Option<String>) = conn.query_row(
            "SELECT platform, base_url FROM api_keys WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        result
    };

    let platform =
        serde_json::from_str::<Platform>(&format!("\"{}\"", platform)).unwrap_or(Platform::Custom);

    // 调用对应平台适配器进行检测
    let status =
        crate::services::validator::check_key_validity(&platform, &secret, base_url.as_deref())
            .await;

    // 更新状态
    let status_str = serde_json::to_string(&status)
        .unwrap_or("\"unknown\"".to_string())
        .trim_matches('"')
        .to_string();

    let conn = db.conn();
    conn.execute(
        "UPDATE api_keys SET status = ?1, last_checked_at = ?2 WHERE id = ?3",
        rusqlite::params![status_str, Utc::now().to_rfc3339(), id],
    )?;

    Ok(status)
}
