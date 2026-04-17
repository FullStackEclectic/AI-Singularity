use crate::db::Database;
use crate::services::gemini_instance_store::{GeminiInstanceRecord, GeminiInstanceStore};
use crate::services::ide_injector::inject_gemini_cli_account_to_root;
use crate::services::provider_current::ProviderCurrentService;
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiInstanceLaunchInfo {
    pub instance_id: String,
    pub user_data_dir: String,
    pub launch_command: String,
}

fn quote_windows_arg(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }
    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '^' | '&' | '|' | '<' | '>' | '%'))
    {
        return format!("\"{}\"", value.replace('"', "\\\""));
    }
    value.to_string()
}

fn parse_extra_args(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(|item| item.to_string()).collect()
}

fn resolve_launch_info(instance: &GeminiInstanceRecord) -> GeminiInstanceLaunchInfo {
    let mut env_prefixes = Vec::new();
    if !instance.is_default {
        let escaped_home = instance.user_data_dir.replace('"', "\"\"");
        env_prefixes.push(format!("set \"GEMINI_CLI_HOME={}\"", escaped_home));
    }
    if let Some(project_id) = instance.project_id.as_deref().filter(|item| !item.trim().is_empty()) {
        let escaped_project = project_id.replace('"', "\"\"");
        env_prefixes.push(format!("set \"GOOGLE_CLOUD_PROJECT={}\"", escaped_project));
    }

    let mut command = if env_prefixes.is_empty() {
        "gemini".to_string()
    } else {
        format!("{} && gemini", env_prefixes.join(" && "))
    };
    for arg in parse_extra_args(&instance.extra_args) {
        if !arg.trim().is_empty() {
            command.push(' ');
            command.push_str(&quote_windows_arg(arg.trim()));
        }
    }

    GeminiInstanceLaunchInfo {
        instance_id: instance.id.clone(),
        user_data_dir: instance.user_data_dir.clone(),
        launch_command: command,
    }
}

fn inject_bound_account_if_needed(
    db: &Database,
    bind_account_id: Option<&str>,
    user_data_dir: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    let Some(bind_account_id) = bind_account_id.filter(|item| !item.trim().is_empty()) else {
        return Ok(());
    };

    let account = db
        .get_all_ide_accounts()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|item| item.id == bind_account_id)
        .ok_or_else(|| format!("未找到绑定的 Gemini 账号：{}", bind_account_id))?;

    inject_gemini_cli_account_to_root(
        &account,
        std::path::Path::new(user_data_dir),
        project_id,
    )
}

#[tauri::command]
pub fn list_gemini_instances() -> Result<Vec<GeminiInstanceRecord>, String> {
    GeminiInstanceStore::list_instances()
}

#[tauri::command]
pub fn get_default_gemini_instance() -> Result<GeminiInstanceRecord, String> {
    GeminiInstanceStore::get_default_instance()
}

#[tauri::command]
pub fn add_gemini_instance(
    name: String,
    user_data_dir: String,
) -> Result<GeminiInstanceRecord, String> {
    GeminiInstanceStore::add_instance(name, user_data_dir)
}

#[tauri::command]
pub fn delete_gemini_instance(id: String) -> Result<(), String> {
    GeminiInstanceStore::delete_instance(&id)
}

#[tauri::command]
pub fn update_gemini_instance_settings(
    id: String,
    extra_args: Option<String>,
    bind_account_id: Option<String>,
    project_id: Option<String>,
    follow_local_account: Option<bool>,
) -> Result<GeminiInstanceRecord, String> {
    if id == "__default__" {
        return GeminiInstanceStore::update_default_settings(
            extra_args,
            Some(bind_account_id),
            Some(project_id),
            follow_local_account,
        );
    }
    GeminiInstanceStore::update_instance_settings(
        &id,
        extra_args,
        Some(bind_account_id),
        Some(project_id),
    )
}

#[tauri::command]
pub fn get_gemini_instance_launch_command(id: String) -> Result<GeminiInstanceLaunchInfo, String> {
    let instance = if id == "__default__" {
        GeminiInstanceStore::get_default_instance()?
    } else {
        GeminiInstanceStore::list_instances()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or("未找到对应的 Gemini 实例".to_string())?
    };
    Ok(resolve_launch_info(&instance))
}

#[tauri::command]
pub fn launch_gemini_instance(
    db: State<'_, Database>,
    id: String,
) -> Result<String, String> {
    let mut instance = if id == "__default__" {
        GeminiInstanceStore::get_default_instance()?
    } else {
        GeminiInstanceStore::list_instances()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or("未找到对应的 Gemini 实例".to_string())?
    };

    if instance.is_default && instance.follow_local_account {
        instance.bind_account_id = ProviderCurrentService::get_current_account_id(&db, "gemini")?;
    }

    inject_bound_account_if_needed(
        &db,
        instance.bind_account_id.as_deref(),
        &instance.user_data_dir,
        instance.project_id.as_deref(),
    )?;

    let launch_info = resolve_launch_info(&instance);
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", "cmd", "/K", &launch_info.launch_command])
            .spawn()
            .map_err(|e| format!("打开终端失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        return Err("当前版本暂未实现 macOS 的 Gemini 实例终端拉起".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        return Err("当前版本暂未实现 Linux 的 Gemini 实例终端拉起".to_string());
    }

    GeminiInstanceStore::update_last_launched(&instance.id)?;
    Ok("已在终端执行 Gemini CLI 命令".to_string())
}
