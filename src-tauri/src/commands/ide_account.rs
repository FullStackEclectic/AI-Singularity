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
