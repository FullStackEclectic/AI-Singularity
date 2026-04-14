use crate::db::Database;
use crate::error::AppResult;
use crate::services::ide_injector::IdeInjector;
use tauri::State;

#[tauri::command]
pub async fn force_inject_ide(account_id: String, db: State<'_, Database>) -> AppResult<()> {
    let target_account = db
        .get_all_ide_accounts()?
        .into_iter()
        .find(|account| account.id == account_id);

    if let Some(acc) = target_account {
        IdeInjector::execute_injection(&acc)?;
        Ok(())
    } else {
        Err(crate::error::AppError::Other(anyhow::anyhow!(
            "未能在兵工厂中找到该账户！"
        )))
    }
}
