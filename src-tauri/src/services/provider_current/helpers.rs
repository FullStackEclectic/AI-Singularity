use crate::models::IdeAccount;
use base64::Engine;
use serde_json::Value;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

pub(super) fn parse_meta(raw: Option<&str>) -> Value {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null)
}

pub(super) fn normalize_optional_string(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn normalize_optional_value(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn ide_state_db_path(app_name: &str) -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").map_err(|_| "无法获取 APPDATA 环境变量".to_string())?;
        return Ok(std::path::PathBuf::from(appdata)
            .join(app_name)
            .join("User")
            .join("globalStorage")
            .join("state.vscdb"));
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        return Ok(home
            .join("Library")
            .join("Application Support")
            .join(app_name)
            .join("User")
            .join("globalStorage")
            .join("state.vscdb"));
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        return Ok(home
            .join(".config")
            .join(app_name)
            .join("User")
            .join("globalStorage")
            .join("state.vscdb"));
    }

    #[allow(unreachable_code)]
    Err(format!("当前平台暂不支持读取 {} 本地数据库", app_name))
}

pub(super) fn app_data_root(app_name: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").map_err(|_| "无法获取 APPDATA 环境变量".to_string())?;
        return Ok(PathBuf::from(appdata).join(app_name));
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        return Ok(home
            .join("Library")
            .join("Application Support")
            .join(app_name));
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        return Ok(home.join(".config").join(app_name));
    }

    #[allow(unreachable_code)]
    Err(format!("当前平台暂不支持读取 {} 数据目录", app_name))
}

pub(super) fn get_default_qoder_state_db_path() -> Result<PathBuf, String> {
    Ok(app_data_root("Qoder")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb"))
}

pub(super) fn get_default_trae_storage_path() -> Result<PathBuf, String> {
    Ok(app_data_root("Trae")?
        .join("User")
        .join("globalStorage")
        .join("storage.json"))
}

#[cfg(target_os = "macos")]
fn security_command_output(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("security")
        .args(args)
        .output()
        .map_err(|e| format!("执行 security 命令失败: {}", e))
}

#[cfg(target_os = "macos")]
fn parse_zed_account_from_security_output(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.split("\"acct\"<blob>=\"").nth(1) {
            if let Some(value) = rest.split('"').next() {
                if let Some(normalized) = normalize_optional_value(Some(value)) {
                    return Some(normalized);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
pub(super) fn read_local_zed_credentials() -> Result<Option<(String, String)>, String> {
    let meta_output =
        security_command_output(&["find-internet-password", "-s", "https://zed.dev"])?;
    if !meta_output.status.success() {
        let stderr = String::from_utf8_lossy(&meta_output.stderr);
        if stderr.contains("could not be found") {
            return Ok(None);
        }
        return Err(format!("读取 Zed Keychain 元数据失败: {}", stderr.trim()));
    }

    let password_output =
        security_command_output(&["find-internet-password", "-s", "https://zed.dev", "-w"])?;
    if !password_output.status.success() {
        let stderr = String::from_utf8_lossy(&password_output.stderr);
        return Err(format!("读取 Zed Keychain 密码失败: {}", stderr.trim()));
    }

    let meta_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&meta_output.stdout),
        String::from_utf8_lossy(&meta_output.stderr)
    );
    let user_id = parse_zed_account_from_security_output(&meta_text)
        .ok_or_else(|| "解析 Zed Keychain 账号失败".to_string())?;
    let access_token = String::from_utf8_lossy(&password_output.stdout)
        .trim()
        .to_string();
    if access_token.is_empty() {
        return Ok(None);
    }

    Ok(Some((user_id, access_token)))
}

#[cfg(not(target_os = "macos"))]
pub(super) fn read_local_zed_credentials() -> Result<Option<(String, String)>, String> {
    Ok(None)
}

pub(super) fn read_db_string(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM ItemTable WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .ok()
    .and_then(|value| normalize_optional_value(Some(value.as_str())))
}

pub(super) fn read_local_kiro_auth_token_json() -> Result<Option<Value>, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let path = home
        .join(".aws")
        .join("sso")
        .join("cache")
        .join("kiro-auth-token.json");
    if !path.exists() {
        return Ok(None);
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("读取 Kiro 本地授权文件失败: {}", e))?;
    let parsed = serde_json::from_str::<Value>(&raw)
        .map_err(|e| format!("解析 Kiro 本地授权文件失败: {}", e))?;
    Ok(Some(parsed))
}

pub(super) fn read_local_kiro_profile_json() -> Result<Option<Value>, String> {
    #[cfg(target_os = "windows")]
    let path = {
        let appdata =
            std::env::var("APPDATA").map_err(|_| "无法获取 APPDATA 环境变量".to_string())?;
        std::path::PathBuf::from(appdata)
            .join("Kiro")
            .join("User")
            .join("globalStorage")
            .join("kiro.kiroagent")
            .join("profile.json")
    };

    #[cfg(target_os = "macos")]
    let path = {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        home.join("Library")
            .join("Application Support")
            .join("Kiro")
            .join("User")
            .join("globalStorage")
            .join("kiro.kiroagent")
            .join("profile.json")
    };

    #[cfg(target_os = "linux")]
    let path = {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        home.join(".config")
            .join("Kiro")
            .join("User")
            .join("globalStorage")
            .join("kiro.kiroagent")
            .join("profile.json")
    };

    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 Kiro profile.json 失败: {}", e))?;
    let parsed = serde_json::from_str::<Value>(&raw)
        .map_err(|e| format!("解析 Kiro profile.json 失败: {}", e))?;
    Ok(Some(parsed))
}

pub(super) fn pick_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = find_string_recursively(value, key) {
            return Some(found);
        }
    }
    None
}

fn find_string_recursively(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(found) = map.get(key).and_then(|item| item.as_str()) {
                return normalize_optional_value(Some(found));
            }
            for nested in map.values() {
                if let Some(found) = find_string_recursively(nested, key) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_string_recursively(item, key)),
        _ => None,
    }
}

pub(super) fn find_matching_ide_account_id(
    accounts: impl Iterator<Item = IdeAccount>,
    email: Option<&str>,
    access_token: Option<&str>,
    refresh_token: Option<&str>,
    user_id: Option<&str>,
) -> Option<String> {
    let email = normalize_optional_value(email);
    let access_token = normalize_optional_value(access_token);
    let refresh_token = normalize_optional_value(refresh_token);
    let user_id = normalize_optional_value(user_id);

    let accounts = accounts.collect::<Vec<_>>();
    for account in &accounts {
        if email
            .as_ref()
            .is_some_and(|expected| account.email.eq_ignore_ascii_case(expected))
        {
            return Some(account.id.clone());
        }
        if access_token
            .as_ref()
            .is_some_and(|expected| account.token.access_token.trim() == expected)
        {
            return Some(account.id.clone());
        }
        if refresh_token
            .as_ref()
            .is_some_and(|expected| account.token.refresh_token.trim() == expected)
        {
            return Some(account.id.clone());
        }
        if user_id.as_ref().is_some_and(|expected| {
            parse_meta(account.meta_json.as_deref())
                .get("user_id")
                .and_then(|value| value.as_str())
                .map(|value| value.trim())
                == Some(expected.as_str())
        }) {
            return Some(account.id.clone());
        }
    }

    accounts
        .into_iter()
        .max_by_key(|account| account.last_used)
        .map(|account| account.id)
}

pub(super) fn decode_jwt_claim(token: &str, claim: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload_b64 = parts[1].replace('-', "+").replace('_', "/");
    let padded = match payload_b64.len() % 4 {
        2 => format!("{}==", payload_b64),
        3 => format!("{}=", payload_b64),
        _ => payload_b64,
    };
    let payload = base64::engine::general_purpose::STANDARD
        .decode(padded)
        .ok()?;
    let value: Value = serde_json::from_slice(&payload).ok()?;
    value.get(claim)?.as_str().map(|value| value.to_string())
}
