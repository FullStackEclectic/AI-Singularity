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

#[cfg(not(target_os = "windows"))]
fn quote_posix_arg(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value.chars().any(|ch| {
        ch.is_whitespace()
            || matches!(
                ch,
                '\'' | '"' | '\\' | '$' | '`' | '!' | '&' | '|' | ';' | '<' | '>'
            )
    }) {
        return format!("'{}'", value.replace('\'', "'\"'\"'"));
    }
    value.to_string()
}

fn parse_extra_args(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(|item| item.to_string())
        .collect()
}

fn resolve_launch_command(instance: &GeminiInstanceRecord) -> String {
    #[cfg(target_os = "windows")]
    {
        let mut env_prefixes = Vec::new();
        if !instance.is_default {
            let escaped_home = instance.user_data_dir.replace('"', "\"\"");
            env_prefixes.push(format!("set \"GEMINI_CLI_HOME={}\"", escaped_home));
        }
        if let Some(project_id) = instance
            .project_id
            .as_deref()
            .filter(|item| !item.trim().is_empty())
        {
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
        return command;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut segments = Vec::new();
        if !instance.is_default {
            segments.push(format!(
                "GEMINI_CLI_HOME={}",
                quote_posix_arg(instance.user_data_dir.trim())
            ));
        }
        if let Some(project_id) = instance
            .project_id
            .as_deref()
            .filter(|item| !item.trim().is_empty())
        {
            segments.push(format!(
                "GOOGLE_CLOUD_PROJECT={}",
                quote_posix_arg(project_id.trim())
            ));
        }
        segments.push("gemini".to_string());
        for arg in parse_extra_args(&instance.extra_args) {
            if !arg.trim().is_empty() {
                segments.push(quote_posix_arg(arg.trim()));
            }
        }
        return segments.join(" ");
    }
}

fn resolve_launch_info(instance: &GeminiInstanceRecord) -> GeminiInstanceLaunchInfo {
    let command = resolve_launch_command(instance);

    GeminiInstanceLaunchInfo {
        instance_id: instance.id.clone(),
        user_data_dir: instance.user_data_dir.clone(),
        launch_command: command,
    }
}

#[cfg(target_os = "macos")]
fn escape_for_osascript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "linux")]
fn launch_linux_terminal(command: &str) -> Result<(), String> {
    let attempts: Vec<(&str, Vec<&str>)> = vec![
        ("x-terminal-emulator", vec!["-e", "bash", "-lc", command]),
        ("gnome-terminal", vec!["--", "bash", "-lc", command]),
        ("konsole", vec!["-e", "bash", "-lc", command]),
        ("xfce4-terminal", vec!["-e", command]),
        ("xterm", vec!["-e", "bash", "-lc", command]),
    ];

    for (program, args) in attempts {
        let result = std::process::Command::new(program).args(args).spawn();
        if result.is_ok() {
            return Ok(());
        }
    }

    Err("未找到可用的 Linux 终端模拟器（已尝试 x-terminal-emulator / gnome-terminal / konsole / xfce4-terminal / xterm）".to_string())
}

#[cfg(target_os = "macos")]
fn launch_macos_terminal(command: &str) -> Result<(), String> {
    std::process::Command::new("osascript")
        .args([
            "-e",
            &format!(
                "tell application \"Terminal\" to do script \"{}\"",
                escape_for_osascript(command)
            ),
            "-e",
            "tell application \"Terminal\" to activate",
        ])
        .spawn()
        .map_err(|e| format!("打开终端失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_terminal(command: &str) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", "cmd", "/K", command])
        .spawn()
        .map_err(|e| format!("打开终端失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_terminal(command: &str) -> Result<(), String> {
    launch_macos_terminal(command)
}

#[cfg(target_os = "linux")]
fn launch_terminal(command: &str) -> Result<(), String> {
    launch_linux_terminal(command)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn launch_terminal(_command: &str) -> Result<(), String> {
    Err("当前平台暂不支持 Gemini 实例终端拉起".to_string())
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

    inject_gemini_cli_account_to_root(&account, std::path::Path::new(user_data_dir), project_id)
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
pub fn launch_gemini_instance(db: State<'_, Database>, id: String) -> Result<String, String> {
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
    launch_terminal(&launch_info.launch_command)?;

    GeminiInstanceStore::update_last_launched(&instance.id)?;
    Ok("已在终端执行 Gemini CLI 命令".to_string())
}
