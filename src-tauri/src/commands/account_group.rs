use crate::db::Database;
use crate::services::account_group_store::{AccountGroup, AccountGroupStore};
use crate::services::event_bus::EventBus;
use tauri::{AppHandle, Manager, State};

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))
}

fn valid_account_ids(db: &Database) -> Result<Vec<String>, String> {
    Ok(db
        .get_all_ide_accounts()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>())
}

fn emit_group_changed(app: &AppHandle, action: &str) {
    crate::tray::update_tray_menu(app);
    EventBus::emit_data_changed(app, "account_groups", action, "account_groups.changed");
}

#[tauri::command]
pub fn list_account_groups(
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<Vec<AccountGroup>, String> {
    AccountGroupStore::list_groups(&app_data_dir(&app)?, &valid_account_ids(&db)?)
}

#[tauri::command]
pub fn create_account_group(
    app: AppHandle,
    db: State<'_, Database>,
    name: String,
) -> Result<AccountGroup, String> {
    let group =
        AccountGroupStore::create_group(&app_data_dir(&app)?, &valid_account_ids(&db)?, &name)?;
    emit_group_changed(&app, "create");
    Ok(group)
}

#[tauri::command]
pub fn rename_account_group(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<AccountGroup, String> {
    let group = AccountGroupStore::rename_group(
        &app_data_dir(&app)?,
        &valid_account_ids(&db)?,
        &id,
        &name,
    )?;
    emit_group_changed(&app, "rename");
    Ok(group)
}

#[tauri::command]
pub fn delete_account_group(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
) -> Result<bool, String> {
    let deleted =
        AccountGroupStore::delete_group(&app_data_dir(&app)?, &valid_account_ids(&db)?, &id)?;
    if deleted {
        emit_group_changed(&app, "delete");
    }
    Ok(deleted)
}

#[tauri::command]
pub fn assign_ide_accounts_to_group(
    app: AppHandle,
    db: State<'_, Database>,
    group_id: String,
    ids: Vec<String>,
) -> Result<AccountGroup, String> {
    let group = AccountGroupStore::assign_accounts_to_group(
        &app_data_dir(&app)?,
        &valid_account_ids(&db)?,
        &group_id,
        &ids,
    )?;
    emit_group_changed(&app, "assign");
    Ok(group)
}

#[tauri::command]
pub fn remove_ide_accounts_from_group(
    app: AppHandle,
    db: State<'_, Database>,
    group_id: String,
    ids: Vec<String>,
) -> Result<AccountGroup, String> {
    let group = AccountGroupStore::remove_accounts_from_group(
        &app_data_dir(&app)?,
        &valid_account_ids(&db)?,
        &group_id,
        &ids,
    )?;
    emit_group_changed(&app, "remove");
    Ok(group)
}
