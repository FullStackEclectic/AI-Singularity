use crate::db::Database;
use crate::{models::Model, AppError};
use tauri::State;

/// 获取所有模型（从缓存）
#[tauri::command]
pub async fn list_models(db: State<'_, Database>) -> Result<Vec<Model>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, platform, name, context_length, supports_vision, supports_tools,
                input_price_per_1m, output_price_per_1m, is_available
         FROM models WHERE is_available = 1 ORDER BY platform, id",
    )?;

    let models = stmt
        .query_map([], |row| {
            use crate::models::Platform;
            let platform =
                serde_json::from_str::<Platform>(&format!("\"{}\"", row.get::<_, String>(1)?))
                    .unwrap_or(Platform::Custom);
            Ok(Model {
                id: row.get(0)?,
                name: row.get(2)?,
                platform,
                context_length: row.get(3)?,
                supports_vision: row.get::<_, i32>(4)? != 0,
                supports_tools: row.get::<_, i32>(5)? != 0,
                input_price_per_1m: row.get(6)?,
                output_price_per_1m: row.get(7)?,
                is_available: row.get::<_, i32>(8)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(models)
}

/// 获取指定平台的模型列表
#[tauri::command]
pub async fn get_platform_models(
    db: State<'_, Database>,
    platform: String,
) -> Result<Vec<Model>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, platform, name, context_length, supports_vision, supports_tools,
                input_price_per_1m, output_price_per_1m, is_available
         FROM models WHERE platform = ?1 AND is_available = 1 ORDER BY id",
    )?;

    let models = stmt
        .query_map(rusqlite::params![platform], |row| {
            use crate::models::Platform;
            let platform =
                serde_json::from_str::<Platform>(&format!("\"{}\"", row.get::<_, String>(1)?))
                    .unwrap_or(Platform::Custom);
            Ok(Model {
                id: row.get(0)?,
                name: row.get(2)?,
                platform,
                context_length: row.get(3)?,
                supports_vision: row.get::<_, i32>(4)? != 0,
                supports_tools: row.get::<_, i32>(5)? != 0,
                input_price_per_1m: row.get(6)?,
                output_price_per_1m: row.get(7)?,
                is_available: row.get::<_, i32>(8)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(models)
}
