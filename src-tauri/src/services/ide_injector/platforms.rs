use super::secret_storage::{
    inject_copilot_token_for_user_data_dir, inject_secret_to_state_db_for_codebuddy,
    inject_secret_to_state_db_for_codebuddy_cn, inject_secret_to_state_db_for_qoder,
    inject_secret_to_state_db_for_workbuddy,
};
use crate::models::IdeAccount;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use std::process::Command;

pub(super) fn inject_platform_account(acc: &IdeAccount) -> Result<(), String> {
    let platform = acc.origin_platform.to_lowercase();
    let access_token = acc.token.access_token.clone();

    if platform.contains("codex") {
        inject_codex_account(acc)
    } else if platform.contains("gemini") {
        inject_gemini_cli_account(acc)
    } else if platform.contains("cursor") {
        inject_cursor_account(acc)
    } else if platform.contains("codebuddy_cn") {
        inject_codebuddy_cn_account(acc)
    } else if platform.contains("codebuddy") {
        inject_codebuddy_account(acc)
    } else if platform.contains("workbuddy") {
        inject_workbuddy_account(acc)
    } else if platform.contains("windsurf") {
        inject_windsurf_account(acc)
    } else if platform.contains("kiro") {
        inject_kiro_account(acc)
    } else if platform.contains("zed") {
        inject_zed_account(acc)
    } else if platform.contains("qoder") {
        inject_qoder_account(acc)
    } else if platform.contains("trae") {
        inject_trae_account(acc)
    } else if platform.contains("copilot") {
        inject_copilot_token_for_user_data_dir("", &acc.email, &access_token, None).map(|_| ())
    } else {
        inject_copilot_token_for_user_data_dir("", &acc.email, &access_token, None).map(|_| ())
    }
}

pub fn inject_gemini_cli_account_to_root(
    acc: &IdeAccount,
    root_dir: &Path,
    project_id_override: Option<&str>,
) -> Result<(), String> {
    let gemini_dir = root_dir.join(".gemini");
    std::fs::create_dir_all(&gemini_dir).map_err(|e| format!("创建 Gemini 目录失败: {}", e))?;

    let expiry_date =
        acc.token.updated_at.timestamp_millis() + (acc.token.expires_in as i64 * 1000);

    let oauth_creds = serde_json::json!({
        "access_token": acc.token.access_token,
        "refresh_token": if acc.token.refresh_token.trim().is_empty() { serde_json::Value::Null } else { serde_json::Value::String(acc.token.refresh_token.clone()) },
        "token_type": if acc.token.token_type.trim().is_empty() { "Bearer" } else { acc.token.token_type.as_str() },
        "expiry_date": expiry_date,
    });
    let oauth_content = serde_json::to_string_pretty(&oauth_creds)
        .map_err(|e| format!("序列化 Gemini oauth_creds.json 失败: {}", e))?;
    std::fs::write(
        gemini_dir.join("oauth_creds.json"),
        format!("{}\n", oauth_content),
    )
    .map_err(|e| format!("写入 Gemini oauth_creds.json 失败: {}", e))?;

    let google_accounts_path = gemini_dir.join("google_accounts.json");
    let existing_accounts = if google_accounts_path.exists() {
        std::fs::read_to_string(&google_accounts_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .unwrap_or_else(|| serde_json::json!({ "active": acc.email, "old": [] }))
    } else {
        serde_json::json!({ "active": acc.email, "old": [] })
    };
    let mut old_accounts = existing_accounts
        .get("old")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if let Some(previous_active) = existing_accounts.get("active").and_then(|value| value.as_str())
    {
        if !previous_active.eq_ignore_ascii_case(&acc.email)
            && !old_accounts
                .iter()
                .any(|item| item.as_str() == Some(previous_active))
        {
            old_accounts.push(serde_json::Value::String(previous_active.to_string()));
        }
    }
    let google_accounts = serde_json::json!({
        "active": acc.email,
        "old": old_accounts,
    });
    let google_accounts_content = serde_json::to_string_pretty(&google_accounts)
        .map_err(|e| format!("序列化 Gemini google_accounts.json 失败: {}", e))?;
    std::fs::write(
        &google_accounts_path,
        format!("{}\n", google_accounts_content),
    )
    .map_err(|e| format!("写入 Gemini google_accounts.json 失败: {}", e))?;

    let settings_path = gemini_dir.join("settings.json");
    let mut settings = if settings_path.exists() {
        std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if !settings.is_object() {
        settings = serde_json::json!({});
    }
    if settings
        .get("security")
        .and_then(|value| value.as_object())
        .is_none()
    {
        settings["security"] = serde_json::json!({});
    }
    if settings["security"]
        .get("auth")
        .and_then(|value| value.as_object())
        .is_none()
    {
        settings["security"]["auth"] = serde_json::json!({});
    }
    settings["security"]["auth"]["selectedType"] = serde_json::json!("oauth-personal");
    let effective_project_id = project_id_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            acc.project_id
                .clone()
                .filter(|value| !value.trim().is_empty())
        });
    if let Some(project_id) = effective_project_id {
        settings["projectId"] = serde_json::json!(project_id);
    }
    let settings_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化 Gemini settings.json 失败: {}", e))?;
    std::fs::write(settings_path, format!("{}\n", settings_content))
        .map_err(|e| format!("写入 Gemini settings.json 失败: {}", e))?;

    Ok(())
}

fn inject_gemini_cli_account(acc: &IdeAccount) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    inject_gemini_cli_account_to_root(acc, &home, None)
}

pub fn inject_codex_account_to_dir(acc: &IdeAccount, codex_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(codex_dir).map_err(|e| format!("创建 Codex 目录失败: {}", e))?;

    let meta = acc
        .meta_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let auth_mode = meta
        .get("auth_mode")
        .and_then(|value| value.as_str())
        .unwrap_or("oauth");
    let openai_api_key = meta
        .get("openai_api_key")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let api_base_url = meta
        .get("api_base_url")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let last_refresh = meta
        .get("last_refresh")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    let tokens = serde_json::json!({
        "access_token": acc.token.access_token,
        "refresh_token": if acc.token.refresh_token.trim().is_empty() { serde_json::Value::Null } else { serde_json::Value::String(acc.token.refresh_token.clone()) },
        "id_token": meta.get("id_token").cloned().unwrap_or(serde_json::Value::Null),
        "account_id": meta.get("account_id").cloned().unwrap_or(serde_json::Value::Null),
    });

    let auth_json = if auth_mode.eq_ignore_ascii_case("apikey") {
        serde_json::json!({
            "OPENAI_API_KEY": openai_api_key,
            "base_url": api_base_url,
            "last_refresh": last_refresh,
            "tokens": tokens,
            "auth_mode": "apikey",
        })
    } else {
        serde_json::json!({
            "OPENAI_API_KEY": openai_api_key,
            "base_url": api_base_url,
            "last_refresh": last_refresh,
            "tokens": tokens,
            "auth_mode": "oauth",
        })
    };

    let auth_content = serde_json::to_string_pretty(&auth_json)
        .map_err(|e| format!("序列化 Codex auth.json 失败: {}", e))?;
    std::fs::write(codex_dir.join("auth.json"), format!("{}\n", auth_content))
        .map_err(|e| format!("写入 Codex auth.json 失败: {}", e))?;

    Ok(())
}

fn inject_codex_account(acc: &IdeAccount) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let codex_dir = home.join(".codex");
    inject_codex_account_to_dir(acc, &codex_dir)
}

fn app_data_root(app_name: &str) -> Result<PathBuf, String> {
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

fn parse_meta_object(acc: &IdeAccount) -> serde_json::Map<String, serde_json::Value> {
    acc.meta_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn inject_qoder_account(acc: &IdeAccount) -> Result<(), String> {
    let db_path = app_data_root("Qoder")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    let meta = parse_meta_object(acc);

    let user_info_json = meta.get("auth_user_info_raw").cloned().unwrap_or_else(|| {
        serde_json::json!({
            "id": meta.get("user_id").cloned().unwrap_or(serde_json::Value::Null),
            "email": acc.email,
            "name": acc.disabled_reason.clone().unwrap_or_default(),
        })
    });
    let user_plan_json = meta
        .get("auth_user_plan_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let credit_usage_json = meta
        .get("auth_credit_usage_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let user_info_raw = serde_json::to_string(&user_info_json)
        .map_err(|e| format!("序列化 Qoder userInfo 失败: {}", e))?;
    let user_plan_raw = serde_json::to_string(&user_plan_json)
        .map_err(|e| format!("序列化 Qoder userPlan 失败: {}", e))?;
    let credit_usage_raw = serde_json::to_string(&credit_usage_json)
        .map_err(|e| format!("序列化 Qoder creditUsage 失败: {}", e))?;

    inject_secret_to_state_db_for_qoder(
        db_path.as_path(),
        "secret://aicoding.auth.userInfo",
        &user_info_raw,
    )?;
    inject_secret_to_state_db_for_qoder(
        db_path.as_path(),
        "secret://aicoding.auth.userPlan",
        &user_plan_raw,
    )?;
    inject_secret_to_state_db_for_qoder(
        db_path.as_path(),
        "secret://aicoding.auth.creditUsage",
        &credit_usage_raw,
    )?;
    Ok(())
}

fn inject_windsurf_account(acc: &IdeAccount) -> Result<(), String> {
    let db_path = app_data_root("Windsurf")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Windsurf 目录失败: {}", e))?;
    }

    let meta = parse_meta_object(acc);
    let mut auth_status = meta
        .get("windsurf_auth_status_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !auth_status.is_object() {
        auth_status = serde_json::json!({});
    }

    if let Some(obj) = auth_status.as_object_mut() {
        if obj.get("apiKey").is_none() && obj.get("api_key").is_none() {
            obj.insert(
                "apiKey".to_string(),
                serde_json::Value::String(acc.token.access_token.clone()),
            );
        }
        if obj.get("email").is_none() {
            obj.insert(
                "email".to_string(),
                serde_json::Value::String(acc.email.clone()),
            );
        }
        if obj.get("name").is_none() && acc.disabled_reason.is_some() {
            obj.insert(
                "name".to_string(),
                serde_json::Value::String(acc.disabled_reason.clone().unwrap_or_default()),
            );
        }
        if let Some(api_server_url) = meta
            .get("windsurf_api_server_url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("apiServerUrl".to_string())
                .or_insert_with(|| serde_json::Value::String(api_server_url.to_string()));
        }
    }

    let raw = serde_json::to_string(&auth_status)
        .map_err(|e| format!("序列化 Windsurf authStatus 失败: {}", e))?;
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("打开 Windsurf state.vscdb 失败: {}", e))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )
    .map_err(|e| format!("初始化 Windsurf ItemTable 失败: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["windsurfAuthStatus", raw],
    )
    .map_err(|e| format!("写入 Windsurf authStatus 失败: {}", e))?;
    Ok(())
}

fn inject_cursor_account(acc: &IdeAccount) -> Result<(), String> {
    let db_path = app_data_root("Cursor")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Cursor 目录失败: {}", e))?;
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("打开 Cursor state.vscdb 失败: {}", e))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )
    .map_err(|e| format!("初始化 Cursor ItemTable 失败: {}", e))?;

    let meta = parse_meta_object(acc);
    let auth_id = meta
        .get("authId")
        .or_else(|| meta.get("auth_id"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string());
    let membership_type = meta
        .get("stripeMembershipType")
        .or_else(|| meta.get("membership_type"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string());
    let subscription_status = meta
        .get("stripeSubscriptionStatus")
        .or_else(|| meta.get("subscription_status"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string());

    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursorAuth/accessToken", acc.token.access_token],
    )
    .map_err(|e| format!("写入 Cursor accessToken 失败: {}", e))?;
    if !acc.token.refresh_token.trim().is_empty() && acc.token.refresh_token != "missing" {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/refreshToken", acc.token.refresh_token],
        )
        .map_err(|e| format!("写入 Cursor refreshToken 失败: {}", e))?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursorAuth/cachedEmail", acc.email],
    )
    .map_err(|e| format!("写入 Cursor cachedEmail 失败: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursor.accessToken", acc.token.access_token],
    )
    .map_err(|e| format!("写入 Cursor cursor.accessToken 失败: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursor.email", acc.email],
    )
    .map_err(|e| format!("写入 Cursor cursor.email 失败: {}", e))?;

    if let Some(auth_id) = auth_id {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/authId", auth_id],
        )
        .map_err(|e| format!("写入 Cursor authId 失败: {}", e))?;
    }
    if let Some(membership_type) = membership_type {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/stripeMembershipType", membership_type],
        )
        .map_err(|e| format!("写入 Cursor stripeMembershipType 失败: {}", e))?;
    }
    if let Some(subscription_status) = subscription_status {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/stripeSubscriptionStatus", subscription_status],
        )
        .map_err(|e| format!("写入 Cursor stripeSubscriptionStatus 失败: {}", e))?;
    }

    Ok(())
}

fn inject_codebuddy_like_account(
    acc: &IdeAccount,
    app_name: &str,
    secret_key: &str,
    writer: fn(&Path, &str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let db_path = app_data_root(app_name)?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    let meta = parse_meta_object(acc);
    let payload = meta
        .get("auth_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({
            "uid": meta.get("uid").cloned().unwrap_or(serde_json::Value::Null),
            "nickname": meta.get("nickname").cloned().unwrap_or(serde_json::Value::Null),
            "email": acc.email,
            "accessToken": acc.token.access_token,
            "refreshToken": if acc.token.refresh_token.trim().is_empty() || acc.token.refresh_token == "missing" {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(acc.token.refresh_token.clone())
            }
        }));
    let raw = serde_json::to_string(&payload)
        .map_err(|e| format!("序列化 {} 登录态失败: {}", app_name, e))?;
    writer(db_path.as_path(), secret_key, &raw)
}

fn inject_codebuddy_account(acc: &IdeAccount) -> Result<(), String> {
    inject_codebuddy_like_account(
        acc,
        "CodeBuddy",
        r#"secret://{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessToken"}"#,
        inject_secret_to_state_db_for_codebuddy,
    )
}

fn inject_codebuddy_cn_account(acc: &IdeAccount) -> Result<(), String> {
    inject_codebuddy_like_account(
        acc,
        "CodeBuddy CN",
        r#"secret://{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessToken"}"#,
        inject_secret_to_state_db_for_codebuddy_cn,
    )
}

fn inject_workbuddy_account(acc: &IdeAccount) -> Result<(), String> {
    inject_codebuddy_like_account(
        acc,
        "WorkBuddy",
        r#"secret://{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessTokencn"}"#,
        inject_secret_to_state_db_for_workbuddy,
    )
}

#[cfg(target_os = "macos")]
fn zed_security_command_output(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("security")
        .args(args)
        .output()
        .map_err(|e| format!("执行 Zed security 命令失败: {}", e))
}

#[cfg(target_os = "macos")]
fn inject_zed_account(acc: &IdeAccount) -> Result<(), String> {
    let meta = parse_meta_object(acc);
    let user_id = meta
        .get("user_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Zed 账号缺少 user_id，无法写入 Keychain".to_string())?;

    loop {
        let output =
            zed_security_command_output(&["delete-internet-password", "-s", "https://zed.dev"])?;
        if output.status.success() {
            continue;
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found") {
            break;
        }
        return Err(format!("删除 Zed Keychain 凭据失败: {}", stderr.trim()));
    }

    let output = zed_security_command_output(&[
        "add-internet-password",
        "-U",
        "-a",
        user_id,
        "-s",
        "https://zed.dev",
        "-w",
        acc.token.access_token.as_str(),
    ])?;
    if !output.status.success() {
        return Err(format!(
            "写入 Zed Keychain 凭据失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn inject_zed_account(_acc: &IdeAccount) -> Result<(), String> {
    Err("Zed 切号当前仅支持 macOS".to_string())
}

fn inject_kiro_account(acc: &IdeAccount) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let auth_path = home
        .join(".aws")
        .join("sso")
        .join("cache")
        .join("kiro-auth-token.json");
    if let Some(parent) = auth_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Kiro 授权目录失败: {}", e))?;
    }

    let profile_path = app_data_root("Kiro")?
        .join("User")
        .join("globalStorage")
        .join("kiro.kiroagent")
        .join("profile.json");
    if let Some(parent) = profile_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 Kiro profile 目录失败: {}", e))?;
    }

    let meta = parse_meta_object(acc);
    let mut auth_json = meta
        .get("kiro_auth_token_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !auth_json.is_object() {
        auth_json = serde_json::json!({});
    }
    if let Some(obj) = auth_json.as_object_mut() {
        obj.insert(
            "accessToken".to_string(),
            serde_json::Value::String(acc.token.access_token.clone()),
        );
        obj.insert(
            "access_token".to_string(),
            serde_json::Value::String(acc.token.access_token.clone()),
        );
        if !acc.token.refresh_token.trim().is_empty() && acc.token.refresh_token != "missing" {
            obj.insert(
                "refreshToken".to_string(),
                serde_json::Value::String(acc.token.refresh_token.clone()),
            );
        }
        if obj.get("email").is_none() {
            obj.insert(
                "email".to_string(),
                serde_json::Value::String(acc.email.clone()),
            );
        }
        if let Some(user_id) = meta
            .get("user_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("userId".to_string())
                .or_insert_with(|| serde_json::Value::String(user_id.to_string()));
        }
    }

    let mut profile_json = meta
        .get("kiro_profile_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !profile_json.is_object() {
        profile_json = serde_json::json!({});
    }
    if let Some(obj) = profile_json.as_object_mut() {
        obj.entry("email".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.email.clone()));
        if let Some(user_id) = meta
            .get("user_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("userId".to_string())
                .or_insert_with(|| serde_json::Value::String(user_id.to_string()));
        }
    }

    let auth_content = serde_json::to_string_pretty(&auth_json)
        .map_err(|e| format!("序列化 Kiro 授权文件失败: {}", e))?;
    std::fs::write(&auth_path, format!("{}\n", auth_content))
        .map_err(|e| format!("写入 Kiro 授权文件失败: {}", e))?;

    let profile_content = serde_json::to_string_pretty(&profile_json)
        .map_err(|e| format!("序列化 Kiro profile.json 失败: {}", e))?;
    std::fs::write(&profile_path, format!("{}\n", profile_content))
        .map_err(|e| format!("写入 Kiro profile.json 失败: {}", e))?;
    Ok(())
}

fn inject_trae_account(acc: &IdeAccount) -> Result<(), String> {
    let storage_path = app_data_root("Trae")?
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    if let Some(parent) = storage_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Trae 目录失败: {}", e))?;
    }

    let meta = parse_meta_object(acc);
    let mut root = meta
        .get("trae_storage_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !root.is_object() {
        root = serde_json::json!({});
    }

    if let Some(obj) = root.as_object_mut() {
        obj.entry("token".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.token.access_token.clone()));
        obj.entry("accessToken".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.token.access_token.clone()));
        if !acc.token.refresh_token.trim().is_empty() && acc.token.refresh_token != "missing" {
            obj.entry("refreshToken".to_string())
                .or_insert_with(|| serde_json::Value::String(acc.token.refresh_token.clone()));
        }
        obj.entry("email".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.email.clone()));
        if let Some(user_id) = meta
            .get("user_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("userId".to_string())
                .or_insert_with(|| serde_json::Value::String(user_id.to_string()));
        }
    }

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 Trae storage.json 失败: {}", e))?;
    std::fs::write(&storage_path, content)
        .map_err(|e| format!("写入 Trae storage.json 失败: {}", e))?;
    Ok(())
}
