use crate::db::Database;
use crate::services::codex_instance_store::{CodexInstanceRecord, CodexInstanceStore};
use crate::services::codex_runtime;
use crate::services::ide_injector::inject_codex_account_to_dir;
use crate::services::provider_current::ProviderCurrentService;
use crate::services::session_manager::{
    ChatMessage, ChatSession, CodexSessionRepairSummary, CodexThreadSyncSummary, SessionManager,
    SessionTrashSummary, ZombieProcess,
};
use crate::services::sync::SyncService;
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_sessions() -> Result<Vec<ChatSession>, String> {
    SessionManager::list_sessions()
}

#[tauri::command]
pub fn get_session_details(filepath: String) -> Result<Vec<ChatMessage>, String> {
    SessionManager::get_session_details(&filepath)
}

#[tauri::command]
pub fn scan_zombies() -> Vec<ZombieProcess> {
    SessionManager::scan_zombie_processes()
}

#[tauri::command]
pub fn launch_session_terminal(cwd: String, command: String) -> Result<(), String> {
    // Attempt to launch wt.exe, fallback to cmd.exe
    // using cmd.exe /c start cmd.exe /K "cd /d cwd && command" to keep it open
    #[cfg(target_os = "windows")]
    {
        let cmd_str = format!("cd /d \"{}\" && {}", cwd, command);
        // We use cmd.exe to start the new window.
        match std::process::Command::new("cmd.exe")
            .args(&["/C", "start", "cmd.exe", "/K", &cmd_str])
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to launch terminal: {}", e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("One-click terminal launch is currently only supported on Windows.".into())
    }
}

#[tauri::command]
pub fn move_sessions_to_trash(filepaths: Vec<String>) -> Result<SessionTrashSummary, String> {
    SessionManager::move_sessions_to_trash(filepaths)
}

#[tauri::command]
pub fn repair_codex_session_index() -> Result<CodexSessionRepairSummary, String> {
    SessionManager::repair_codex_session_index()
}

#[tauri::command]
pub fn sync_codex_threads_across_instances() -> Result<CodexThreadSyncSummary, String> {
    SessionManager::sync_codex_threads_across_instances()
}

#[tauri::command]
pub fn list_codex_instances() -> Result<Vec<CodexInstanceRecord>, String> {
    CodexInstanceStore::list_instances()
}

#[tauri::command]
pub fn get_default_codex_instance() -> Result<CodexInstanceRecord, String> {
    CodexInstanceStore::get_default_instance()
}

fn find_codex_instance(id: &str) -> Result<CodexInstanceRecord, String> {
    if id == "__default__" {
        return CodexInstanceStore::get_default_instance();
    }
    CodexInstanceStore::list_instances()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("未找到对应的 Codex 实例".to_string())
}

#[tauri::command]
pub fn sync_codex_instance_shared_resources(id: String) -> Result<CodexInstanceRecord, String> {
    let instance = find_codex_instance(&id)?;
    crate::services::codex_shared::ensure_instance_shared_resources(std::path::Path::new(
        &instance.user_data_dir,
    ))?;
    find_codex_instance(&id)
}

#[tauri::command]
pub fn add_codex_instance(
    name: String,
    user_data_dir: String,
) -> Result<CodexInstanceRecord, String> {
    crate::services::codex_shared::ensure_instance_shared_resources(std::path::Path::new(
        &user_data_dir,
    ))?;
    CodexInstanceStore::add_instance(name, user_data_dir)
}

#[tauri::command]
pub fn delete_codex_instance(app: AppHandle, id: String) -> Result<(), String> {
    CodexInstanceStore::delete_instance(&id)?;
    let _ = crate::commands::floating_account_card::reconcile_floating_cards_instance_bindings(
        &app,
        "codex_instance.delete",
    );
    Ok(())
}

fn inject_bound_account_if_needed(
    db: &Database,
    bind_account_id: Option<&str>,
    user_data_dir: &str,
) -> Result<(), String> {
    let Some(bind_account_id) = bind_account_id.filter(|item| !item.trim().is_empty()) else {
        return Ok(());
    };

    let account = db
        .get_all_ide_accounts()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|item| item.id == bind_account_id)
        .ok_or_else(|| format!("未找到绑定的 Codex 账号：{}", bind_account_id))?;

    inject_codex_account_to_dir(&account, Path::new(user_data_dir))
}

#[tauri::command]
pub fn update_codex_instance_settings(
    id: String,
    extra_args: Option<String>,
    bind_account_id: Option<String>,
    bind_provider_id: Option<String>,
    follow_local_account: Option<bool>,
) -> Result<CodexInstanceRecord, String> {
    if id == "__default__" {
        return CodexInstanceStore::update_default_settings(
            extra_args,
            Some(bind_account_id),
            Some(bind_provider_id),
            follow_local_account,
        );
    }
    CodexInstanceStore::update_instance_settings(
        &id,
        extra_args,
        Some(bind_account_id),
        Some(bind_provider_id),
    )
}

#[tauri::command]
pub fn start_codex_instance(
    db: State<'_, Database>,
    id: String,
) -> Result<CodexInstanceRecord, String> {
    let mut instance = find_codex_instance(&id)?;

    codex_runtime::validate_user_data_dir(&instance.user_data_dir)?;
    if instance.is_default && instance.follow_local_account {
        instance.bind_account_id = ProviderCurrentService::get_current_account_id(&db, "codex")?;
    }
    crate::services::codex_shared::ensure_instance_shared_resources(std::path::Path::new(
        &instance.user_data_dir,
    ))?;
    SyncService::new(&db).sync_codex_dir_with_provider_id(
        &std::path::PathBuf::from(&instance.user_data_dir),
        instance.bind_provider_id.as_deref(),
    )?;
    inject_bound_account_if_needed(
        &db,
        instance.bind_account_id.as_deref(),
        &instance.user_data_dir,
    )?;

    if let Some(pid) = instance
        .last_pid
        .filter(|pid| codex_runtime::is_pid_running(*pid))
    {
        let _ = codex_runtime::stop_pid(pid);
    }

    let pid = codex_runtime::start_codex_instance(&instance.user_data_dir, &instance.extra_args)?;
    if instance.is_default {
        CodexInstanceStore::set_default_pid(Some(pid))
    } else {
        CodexInstanceStore::set_instance_pid(&instance.id, Some(pid))
    }
}

#[tauri::command]
pub fn stop_codex_instance(id: String) -> Result<CodexInstanceRecord, String> {
    let instance = find_codex_instance(&id)?;

    if let Some(pid) = instance
        .last_pid
        .filter(|pid| codex_runtime::is_pid_running(*pid))
    {
        codex_runtime::stop_pid(pid)?;
    }

    if instance.is_default {
        CodexInstanceStore::set_default_pid(None)
    } else {
        CodexInstanceStore::set_instance_pid(&instance.id, None)
    }
}

#[tauri::command]
pub fn open_codex_instance_window(id: String) -> Result<(), String> {
    let instance = find_codex_instance(&id)?;

    let pid = instance
        .last_pid
        .filter(|pid| codex_runtime::is_pid_running(*pid))
        .ok_or("该 Codex 实例当前未运行".to_string())?;

    codex_runtime::focus_pid(pid)
}

#[tauri::command]
pub fn close_all_codex_instances() -> Result<(), String> {
    let default_instance = CodexInstanceStore::get_default_instance()?;
    if let Some(pid) = default_instance
        .last_pid
        .filter(|pid| codex_runtime::is_pid_running(*pid))
    {
        let _ = codex_runtime::stop_pid(pid);
    }

    for instance in CodexInstanceStore::list_instances()? {
        if let Some(pid) = instance
            .last_pid
            .filter(|pid| codex_runtime::is_pid_running(*pid))
        {
            let _ = codex_runtime::stop_pid(pid);
        }
    }

    CodexInstanceStore::clear_all_pids()?;
    Ok(())
}
