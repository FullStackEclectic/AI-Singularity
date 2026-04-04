use crate::{
    db::Database,
    models::{ApiKey, KeyStatus, Platform},
    store::SecureStore,
    AppError,
};
use chrono::Utc;
use serde::Deserialize;
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
    pub notes: Option<String>,
    pub priority: Option<i64>,
}

/// 获取所有 Key 列表（不含 secret）
#[tauri::command]
pub async fn list_keys(db: State<'_, Database>) -> Result<Vec<ApiKey>, AppError> {
    type Row = (String, String, String, Option<String>, String, String, Option<String>, String, Option<String>, i64);
    let rows: Vec<Row> = db.query_rows(
        "SELECT id, name, platform, base_url, key_preview, status, notes, created_at, last_checked_at, COALESCE(priority, 100)
         FROM api_keys ORDER BY priority DESC, created_at DESC",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
    )?;

    Ok(rows.into_iter().map(|(id, name, platform_str, base_url, key_preview, status_str, notes, created_at, last_checked_at, priority)| {
        ApiKey {
            id, name,
            platform: serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom),
            base_url, key_preview,
            status: serde_json::from_str::<KeyStatus>(&format!("\"{}\"", status_str)).unwrap_or(KeyStatus::Unknown),
            notes,
            created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
            last_checked_at: last_checked_at.and_then(|s| s.parse().ok()),
            priority,
        }
    }).collect())
}

/// 添加新 Key
#[tauri::command]
pub async fn add_key(db: State<'_, Database>, request: AddKeyRequest) -> Result<ApiKey, AppError> {
    let id = Uuid::new_v4().to_string();
    let preview = SecureStore::key_preview(&request.secret);
    let now = Utc::now();

    SecureStore::store_key(&id, &request.secret)?;

    let platform_str = serde_json::to_string(&request.platform)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    db.execute(
        "INSERT INTO api_keys (id, name, platform, base_url, key_hash, key_preview, status, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unknown', ?7, ?8)",
        &[&id, &request.name, &platform_str, &request.base_url as &dyn rusqlite::ToSql,
          &"placeholder", &preview, &request.notes as &dyn rusqlite::ToSql, &now.to_rfc3339()],
    )?;

    Ok(ApiKey {
        id, name: request.name, platform: request.platform, base_url: request.base_url,
        key_preview: preview, status: KeyStatus::Unknown, notes: request.notes,
        created_at: now, last_checked_at: None, priority: 100,
    })
}

/// 删除 Key
#[tauri::command]
pub async fn delete_key(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    SecureStore::delete_key(&id).ok();
    db.execute("DELETE FROM api_keys WHERE id = ?1", &[&id])?;
    Ok(())
}

/// 更新 Key 元数据
#[tauri::command]
pub async fn update_key(db: State<'_, Database>, request: UpdateKeyRequest) -> Result<(), AppError> {
    if let Some(ref secret) = request.secret {
        SecureStore::store_key(&request.id, secret)?;
        let preview = SecureStore::key_preview(secret);
        db.execute("UPDATE api_keys SET key_preview = ?1 WHERE id = ?2", &[&preview, &request.id])?;
    }
    if let Some(ref name) = request.name {
        db.execute("UPDATE api_keys SET name = ?1 WHERE id = ?2", &[name, &request.id])?;
    }
    if let Some(ref notes) = request.notes {
        db.execute("UPDATE api_keys SET notes = ?1 WHERE id = ?2", &[notes, &request.id])?;
    }
    if let Some(priority) = request.priority {
        db.execute("UPDATE api_keys SET priority = ?1 WHERE id = ?2", &[&priority, &request.id])?;
    }
    Ok(())
}

/// 检测 Key 有效性（调用平台 API）
#[tauri::command]
pub async fn check_key(db: State<'_, Database>, id: String) -> Result<KeyStatus, AppError> {
    let secret = SecureStore::get_key(&id)?;

    let (platform_str, base_url): (String, Option<String>) = db.query_one(
        "SELECT platform, base_url FROM api_keys WHERE id = ?1",
        &[&id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str))
        .unwrap_or(Platform::Custom);

    let status = crate::services::validator::check_key_validity(&platform, &secret, base_url.as_deref()).await;

    let status_str = serde_json::to_string(&status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    db.execute(
        "UPDATE api_keys SET status = ?1, last_checked_at = ?2 WHERE id = ?3",
        &[&status_str, &Utc::now().to_rfc3339(), &id],
    )?;

    Ok(status)
}
