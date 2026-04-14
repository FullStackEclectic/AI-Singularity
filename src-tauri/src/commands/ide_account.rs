use crate::db::Database;
use crate::error::AppResult;
use crate::models::IdeAccount;
use crate::services::codex_ide::CodexIdeService;
use crate::services::event_bus::EventBus;
use crate::services::gemini_ide::{GeminiCloudProject, GeminiIdeService};
use crate::services::local_ide_refresh::LocalIdeRefreshService;
use tauri::{AppHandle, State};

#[derive(serde::Serialize)]
struct ExportIdeAccount {
    email: String,
    refresh_token: String,
    access_token: String,
    origin_platform: String,
    meta_json: Option<String>,
    label: Option<String>,
}

#[tauri::command]
pub async fn get_all_ide_accounts(db: State<'_, Database>) -> AppResult<Vec<IdeAccount>> {
    let accounts = db.get_all_ide_accounts()?;
    Ok(accounts)
}

#[tauri::command]
pub async fn import_ide_accounts(
    app: AppHandle,
    db: State<'_, Database>,
    accounts: Vec<IdeAccount>,
) -> AppResult<usize> {
    let mut count = 0;
    for acc in accounts {
        if db.upsert_ide_account(&acc).is_ok() {
            count += 1;
        }
    }
    if count > 0 {
        EventBus::emit_data_changed(&app, "ide_accounts", "import", "ide_account.import");
    }
    Ok(count)
}

#[tauri::command]
pub async fn delete_ide_account(app: AppHandle, db: State<'_, Database>, id: String) -> AppResult<usize> {
    let count = db.delete_ide_account(&id)?;
    if count > 0 {
        EventBus::emit_data_changed(&app, "ide_accounts", "delete", "ide_account.delete");
    }
    Ok(count)
}

/// 更新 IDE 账号标签列表
#[tauri::command]
pub async fn update_ide_account_tags(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    tags: Vec<String>,
) -> AppResult<()> {
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
    db.update_ide_account_tags(&id, &tags_json)?;
    EventBus::emit_data_changed(&app, "ide_accounts", "update_tags", "ide_account.update_tags");
    Ok(())
}

#[tauri::command]
pub async fn update_ide_account_label(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    label: Option<String>,
) -> AppResult<()> {
    db.update_ide_account_label(&id, label.as_deref())?;
    EventBus::emit_data_changed(&app, "ide_accounts", "update_label", "ide_account.update_label");
    Ok(())
}

/// 更新 API Key 标签列表
#[tauri::command]
pub async fn update_api_key_tags(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    tags: Vec<String>,
) -> AppResult<()> {
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
    db.update_api_key_tags(&id, &tags_json)?;
    EventBus::emit_data_changed(&app, "api_keys", "update_tags", "api_key.update_tags");
    Ok(())
}

#[tauri::command]
pub async fn refresh_ide_account(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
) -> Result<IdeAccount, String> {
    let account = db
        .get_all_ide_accounts()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "IDE 账号不存在".to_string())?;

    let refreshed = match account.origin_platform.to_ascii_lowercase().as_str() {
        "gemini" => GeminiIdeService::refresh_account(&*db, &account.id).await,
        "codex" => CodexIdeService::refresh_account(&*db, &account.id).await,
        "cursor" => LocalIdeRefreshService::refresh_cursor_account(&*db, &account.id),
        "windsurf" => LocalIdeRefreshService::refresh_windsurf_account(&*db, &account.id),
        "kiro" => LocalIdeRefreshService::refresh_kiro_account(&*db, &account.id),
        "qoder" => LocalIdeRefreshService::refresh_qoder_account(&*db, &account.id),
        "trae" => LocalIdeRefreshService::refresh_trae_account(&*db, &account.id),
        other => Err(format!("{} 暂不支持刷新账号状态", other)),
    }?;
    EventBus::emit_data_changed(&app, "ide_accounts", "refresh", "ide_account.refresh");
    Ok(refreshed)
}

#[tauri::command]
pub async fn refresh_all_ide_accounts_by_platform(
    app: AppHandle,
    db: State<'_, Database>,
    platform: String,
) -> Result<usize, String> {
    let count = match platform.to_ascii_lowercase().as_str() {
        "gemini" => GeminiIdeService::refresh_all_accounts(&*db).await,
        "codex" => CodexIdeService::refresh_all_accounts(&*db).await,
        "cursor" => LocalIdeRefreshService::refresh_all_by_platform(&*db, "cursor"),
        "windsurf" => LocalIdeRefreshService::refresh_all_by_platform(&*db, "windsurf"),
        "kiro" => LocalIdeRefreshService::refresh_all_by_platform(&*db, "kiro"),
        "qoder" => LocalIdeRefreshService::refresh_all_by_platform(&*db, "qoder"),
        "trae" => LocalIdeRefreshService::refresh_all_by_platform(&*db, "trae"),
        other => Err(format!("{} 暂不支持批量刷新", other)),
    }?;
    if count > 0 {
        EventBus::emit_data_changed(&app, "ide_accounts", "refresh_all", "ide_account.refresh_all");
    }
    Ok(count)
}

#[tauri::command]
pub async fn list_gemini_cloud_projects_for_ide_account(
    db: State<'_, Database>,
    id: String,
) -> Result<Vec<GeminiCloudProject>, String> {
    GeminiIdeService::list_cloud_projects(&*db, &id).await
}

#[tauri::command]
pub async fn set_gemini_project_for_ide_account(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    project_id: Option<String>,
) -> Result<IdeAccount, String> {
    let account = GeminiIdeService::set_project_id(&*db, &id, project_id.as_deref())?;
    EventBus::emit_data_changed(&app, "ide_accounts", "set_project", "ide_account.set_gemini_project");
    Ok(account)
}

#[tauri::command]
pub async fn update_codex_api_key_credentials_for_ide_account(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    api_key: String,
    api_base_url: Option<String>,
) -> Result<IdeAccount, String> {
    let account = CodexIdeService::update_api_key_credentials(&*db, &id, &api_key, api_base_url.as_deref())?;
    EventBus::emit_data_changed(&app, "ide_accounts", "update_codex_api_key", "ide_account.update_codex_api_key");
    Ok(account)
}

#[tauri::command]
pub async fn export_ide_accounts(
    db: State<'_, Database>,
    ids: Vec<String>,
) -> Result<String, String> {
    let accounts = db.get_all_ide_accounts().map_err(|e| e.to_string())?;
    let filtered = if ids.is_empty() {
        accounts
    } else {
        accounts
            .into_iter()
            .filter(|account| ids.iter().any(|id| id == &account.id))
            .collect()
    };

    let export_payload = filtered
        .into_iter()
        .map(|account| ExportIdeAccount {
            email: account.email,
            refresh_token: account.token.refresh_token,
            access_token: account.token.access_token,
            origin_platform: account.origin_platform,
            meta_json: account.meta_json,
            label: account.label,
        })
        .collect::<Vec<_>>();

    serde_json::to_string_pretty(&export_payload).map_err(|e| e.to_string())
}
