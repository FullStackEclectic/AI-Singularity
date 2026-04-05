use crate::db::Database;
use crate::error::AppResult;
use crate::models::IdeAccount;
use tauri::State;

#[tauri::command]
pub async fn get_all_ide_accounts(db: State<'_, Database>) -> AppResult<Vec<IdeAccount>> {
    let accounts = db.get_all_ide_accounts()?;
    Ok(accounts)
}

#[tauri::command]
pub async fn import_ide_accounts(
    db: State<'_, Database>,
    accounts: Vec<IdeAccount>,
) -> AppResult<usize> {
    let mut count = 0;
    for acc in accounts {
        if db.upsert_ide_account(&acc).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub async fn delete_ide_account(db: State<'_, Database>, id: String) -> AppResult<usize> {
    let count = db.delete_ide_account(&id)?;
    Ok(count)
}

/// 更新 IDE 账号标签列表
#[tauri::command]
pub async fn update_ide_account_tags(
    db: State<'_, Database>,
    id: String,
    tags: Vec<String>,
) -> AppResult<()> {
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
    db.update_ide_account_tags(&id, &tags_json)?;
    Ok(())
}

/// 更新 API Key 标签列表
#[tauri::command]
pub async fn update_api_key_tags(
    db: State<'_, Database>,
    id: String,
    tags: Vec<String>,
) -> AppResult<()> {
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
    db.update_api_key_tags(&id, &tags_json)?;
    Ok(())
}
