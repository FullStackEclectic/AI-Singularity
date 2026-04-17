use crate::db::Database;
use crate::error::AppResult;
use crate::services::event_bus::EventBus;
use crate::services::ide_injector::IdeInjector;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn force_inject_ide(app: AppHandle, account_id: String, db: State<'_, Database>) -> AppResult<()> {
    let target_account = db
        .get_all_ide_accounts()?
        .into_iter()
        .find(|account| account.id == account_id);

    if let Some(acc) = target_account {
        IdeInjector::execute_injection(&acc)?;
        crate::commands::floating_account_card::emit_floating_account_changed(
            &app,
            &acc.origin_platform,
            Some(&acc.id),
            "ide_account.force_inject",
        );
        crate::tray::update_tray_menu(&app);
        EventBus::emit_data_changed(&app, "ide_accounts", "force_inject", "ide_account.force_inject");
        Ok(())
    } else {
        Err(crate::error::AppError::Other(anyhow::anyhow!(
            "未能在兵工厂中找到该账户！"
        )))
    }
}
