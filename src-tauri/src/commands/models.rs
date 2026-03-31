use crate::{models::Model, AppError};
use tauri::State;
use crate::db::Database;

/// 获取所有模型（从缓存）
#[tauri::command]
pub async fn list_models(db: State<'_, Database>) -> Result<Vec<Model>, AppError> {
    type Row = (String, String, String, Option<i64>, i32, i32, Option<f64>, Option<f64>, i32);
    let rows: Vec<Row> = db.query_rows(
        "SELECT id, platform, name, context_length, supports_vision, supports_tools,
                input_price_per_1m, output_price_per_1m, is_available
         FROM models WHERE is_available = 1 ORDER BY platform, id",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
    )?;

    use crate::models::Platform;
    Ok(rows.into_iter().map(|(id, platform_str, name, context_length, supports_vision, supports_tools, input_price_per_1m, output_price_per_1m, is_available)| {
        Model {
            id, name,
            platform: serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom),
            context_length: context_length.map(|v| v as u64),
            supports_vision: supports_vision != 0,
            supports_tools: supports_tools != 0,
            input_price_per_1m,
            output_price_per_1m,
            is_available: is_available != 0,
        }
    }).collect())
}

/// 获取指定平台的模型列表
#[tauri::command]
pub async fn get_platform_models(
    db: State<'_, Database>,
    platform: String,
) -> Result<Vec<Model>, AppError> {
    type Row = (String, String, String, Option<i64>, i32, i32, Option<f64>, Option<f64>, i32);
    let rows: Vec<Row> = db.query_rows(
        "SELECT id, platform, name, context_length, supports_vision, supports_tools,
                input_price_per_1m, output_price_per_1m, is_available
         FROM models WHERE platform = ?1 AND is_available = 1 ORDER BY id",
        &[&platform],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
    )?;

    use crate::models::Platform;
    Ok(rows.into_iter().map(|(id, platform_str, name, context_length, supports_vision, supports_tools, input_price_per_1m, output_price_per_1m, is_available)| {
        Model {
            id, name,
            platform: serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom),
            context_length: context_length.map(|v| v as u64),
            supports_vision: supports_vision != 0,
            supports_tools: supports_tools != 0,
            input_price_per_1m,
            output_price_per_1m,
            is_available: is_available != 0,
        }
    }).collect())
}
