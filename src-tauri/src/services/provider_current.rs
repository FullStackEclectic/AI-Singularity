use crate::db::Database;
use crate::models::IdeAccount;
use crate::services::ide_injector::{
    read_codebuddy_cn_secret_storage_value, read_codebuddy_secret_storage_value,
    read_qoder_secret_storage_value_by_db_path, read_workbuddy_secret_storage_value,
};
use base64::Engine;
use std::path::PathBuf;
use serde_json::Value;
#[cfg(target_os = "macos")]
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentAccountSnapshot {
    pub platform: String,
    pub account_id: Option<String>,
    pub label: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
}

pub struct ProviderCurrentService;

impl ProviderCurrentService {
    pub fn get_current_account_id(db: &Database, platform: &str) -> Result<Option<String>, String> {
        match platform.trim().to_ascii_lowercase().as_str() {
            "codex" => Self::get_current_codex_account_id(db),
            "gemini" => Self::get_current_gemini_account_id(db),
            "cursor" => Self::get_current_cursor_account_id(db),
            "windsurf" => Self::get_current_windsurf_account_id(db),
            "kiro" => Self::get_current_kiro_account_id(db),
            "codebuddy" => Self::get_current_codebuddy_account_id(db),
            "codebuddy_cn" => Self::get_current_codebuddy_cn_account_id(db),
            "workbuddy" => Self::get_current_workbuddy_account_id(db),
            "qoder" => Self::get_current_qoder_account_id(db),
            "trae" => Self::get_current_trae_account_id(db),
            "zed" => Self::get_current_zed_account_id(db),
            other => Err(format!("当前暂不支持解析 {} 的当前账号", other)),
        }
    }

    pub fn list_current_account_snapshots(
        db: &Database,
    ) -> Result<Vec<CurrentAccountSnapshot>, String> {
        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;
        let platforms = [
            "codex",
            "gemini",
            "cursor",
            "windsurf",
            "kiro",
            "codebuddy",
            "codebuddy_cn",
            "workbuddy",
            "qoder",
            "trae",
            "zed",
        ];

        let mut snapshots = Vec::new();
        for platform in platforms {
            let account_id = Self::get_current_account_id(db, platform)?;
            let matched = account_id
                .as_ref()
                .and_then(|id| accounts.iter().find(|item| item.id == *id));
            snapshots.push(CurrentAccountSnapshot {
                platform: platform.to_string(),
                account_id: account_id.clone(),
                label: matched.and_then(|item| item.label.clone()).or_else(|| matched.map(|item| item.email.clone())),
                email: matched.map(|item| item.email.clone()),
                status: matched.map(|item| format!("{:?}", item.status).to_lowercase()),
            });
        }
        Ok(snapshots)
    }

    fn get_current_codex_account_id(db: &Database) -> Result<Option<String>, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let auth_path = home.join(".codex").join("auth.json");
        if !auth_path.exists() {
            return Ok(None);
        }

        let raw =
            std::fs::read_to_string(&auth_path).map_err(|e| format!("读取 Codex auth.json 失败: {}", e))?;
        let json: Value =
            serde_json::from_str(&raw).map_err(|e| format!("解析 Codex auth.json 失败: {}", e))?;
        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        let auth_mode = json
            .get("auth_mode")
            .and_then(|value| value.as_str())
            .unwrap_or("oauth");

        if auth_mode.eq_ignore_ascii_case("apikey") {
            let openai_api_key = json
                .get("OPENAI_API_KEY")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let base_url = json
                .get("base_url")
                .or_else(|| json.get("openai_base_url"))
                .and_then(|value| value.as_str())
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty());

            for account in accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("codex"))
            {
                let meta = parse_meta(account.meta_json.as_deref());
                let account_mode = meta
                    .get("auth_mode")
                    .and_then(|value| value.as_str())
                    .unwrap_or("oauth");
                if !account_mode.eq_ignore_ascii_case("apikey") {
                    continue;
                }

                let same_key = meta
                    .get("openai_api_key")
                    .and_then(|value| value.as_str())
                    .map(|value| value.trim())
                    == openai_api_key.as_deref();
                let same_base = normalize_optional_string(meta.get("api_base_url").and_then(|value| value.as_str()))
                    == base_url.clone();

                if same_key && (base_url.is_none() || same_base) {
                    return Ok(Some(account.id));
                }
            }
            return Ok(None);
        }

        let tokens = json.get("tokens").cloned().unwrap_or(Value::Null);
        let account_id = tokens
            .get("account_id")
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let access_token = tokens
            .get("access_token")
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let refresh_token = tokens
            .get("refresh_token")
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let email = tokens
            .get("id_token")
            .and_then(|value| value.as_str())
            .and_then(|token| decode_jwt_claim(token, "email"))
            .or_else(|| access_token.as_deref().and_then(|token| decode_jwt_claim(token, "email")));

        for account in accounts
            .into_iter()
            .filter(|item| item.origin_platform.eq_ignore_ascii_case("codex"))
        {
            let meta = parse_meta(account.meta_json.as_deref());
            if account_id.as_ref().is_some_and(|expected| {
                meta.get("account_id")
                    .and_then(|value| value.as_str())
                    .map(|value| value.trim())
                    == Some(expected.as_str())
            }) {
                return Ok(Some(account.id));
            }
            if email
                .as_ref()
                .is_some_and(|expected| account.email.eq_ignore_ascii_case(expected))
            {
                return Ok(Some(account.id));
            }
            if refresh_token
                .as_ref()
                .is_some_and(|expected| account.token.refresh_token.trim() == expected)
            {
                return Ok(Some(account.id));
            }
            if access_token
                .as_ref()
                .is_some_and(|expected| account.token.access_token.trim() == expected)
            {
                return Ok(Some(account.id));
            }
        }

        Ok(None)
    }

    fn get_current_gemini_account_id(db: &Database) -> Result<Option<String>, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let gemini_dir = home.join(".gemini");
        let oauth_path = gemini_dir.join("oauth_creds.json");
        if !oauth_path.exists() {
            return Ok(None);
        }

        let oauth_raw = std::fs::read_to_string(&oauth_path)
            .map_err(|e| format!("读取 Gemini oauth_creds.json 失败: {}", e))?;
        let oauth_json: Value = serde_json::from_str(&oauth_raw)
            .map_err(|e| format!("解析 Gemini oauth_creds.json 失败: {}", e))?;
        let access_token = oauth_json
            .get("access_token")
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let refresh_token = oauth_json
            .get("refresh_token")
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let active_email = gemini_dir
            .join("google_accounts.json")
            .exists()
            .then(|| {
                std::fs::read_to_string(gemini_dir.join("google_accounts.json"))
                    .ok()
                    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                    .and_then(|json| json.get("active").and_then(|value| value.as_str()).map(|value| value.to_string()))
            })
            .flatten();

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        for account in accounts
            .into_iter()
            .filter(|item| item.origin_platform.eq_ignore_ascii_case("gemini"))
        {
            if active_email
                .as_ref()
                .is_some_and(|expected| account.email.eq_ignore_ascii_case(expected))
            {
                return Ok(Some(account.id));
            }
            if refresh_token
                .as_ref()
                .is_some_and(|expected| account.token.refresh_token.trim() == expected)
            {
                return Ok(Some(account.id));
            }
            if access_token
                .as_ref()
                .is_some_and(|expected| account.token.access_token.trim() == expected)
            {
                return Ok(Some(account.id));
            }
        }

        Ok(None)
    }

    fn get_current_cursor_account_id(db: &Database) -> Result<Option<String>, String> {
        let db_path = ide_state_db_path("Cursor")?;
        if !db_path.exists() {
            return Ok(None);
        }
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("打开 Cursor 本地数据库失败: {}", e))?;
        let access_token = read_db_string(&conn, "cursorAuth/accessToken");
        let email = read_db_string(&conn, "cursorAuth/cachedEmail");

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case("cursor")),
            email.as_deref(),
            access_token.as_deref(),
            None,
            None,
        ))
    }

    fn get_current_windsurf_account_id(db: &Database) -> Result<Option<String>, String> {
        let db_path = ide_state_db_path("Windsurf")?;
        if !db_path.exists() {
            return Ok(None);
        }
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("打开 Windsurf 本地数据库失败: {}", e))?;
        let auth_status = read_db_string(&conn, "windsurfAuthStatus")
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
        let api_key = auth_status
            .as_ref()
            .and_then(|value| pick_string(value, &["apiKey", "api_key"]));
        let email = auth_status
            .as_ref()
            .and_then(|value| pick_string(value, &["email"]));

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case("windsurf")),
            email.as_deref(),
            api_key.as_deref(),
            api_key.as_deref(),
            None,
        ))
    }

    fn get_current_kiro_account_id(db: &Database) -> Result<Option<String>, String> {
        let local_auth = read_local_kiro_auth_token_json()?;
        let local_profile = read_local_kiro_profile_json()?;

        let email = local_profile
            .as_ref()
            .and_then(|value| pick_string(value, &["email", "userEmail"]))
            .or_else(|| local_auth.as_ref().and_then(|value| pick_string(value, &["email", "upn", "preferred_username"])));
        let user_id = local_profile
            .as_ref()
            .and_then(|value| pick_string(value, &["userId", "user_id", "sub", "accountId"]))
            .or_else(|| local_auth.as_ref().and_then(|value| pick_string(value, &["userId", "user_id", "sub"])));
        let refresh_token = local_auth
            .as_ref()
            .and_then(|value| pick_string(value, &["refreshToken", "refresh_token", "refreshTokenJwt"]));

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case("kiro")),
            email.as_deref(),
            None,
            refresh_token.as_deref(),
            user_id.as_deref(),
        ))
    }

    fn get_current_codebuddy_account_id(db: &Database) -> Result<Option<String>, String> {
        let data_root = app_data_root("CodeBuddy")?;
        let secret = read_codebuddy_secret_storage_value(
            "tencent-cloud.coding-copilot",
            "planning-genie.new.accessToken",
            Some(data_root.to_string_lossy().as_ref()),
        )?;
        let Some(secret) = secret else {
            return Ok(None);
        };
        Self::match_codebuddy_like_current_account(db, "codebuddy", &secret)
    }

    fn get_current_codebuddy_cn_account_id(db: &Database) -> Result<Option<String>, String> {
        let data_root = app_data_root("CodeBuddy CN")?;
        let secret = read_codebuddy_cn_secret_storage_value(
            "tencent-cloud.coding-copilot",
            "planning-genie.new.accessToken",
            Some(data_root.to_string_lossy().as_ref()),
        )?;
        let Some(secret) = secret else {
            return Ok(None);
        };
        Self::match_codebuddy_like_current_account(db, "codebuddy_cn", &secret)
    }

    fn get_current_workbuddy_account_id(db: &Database) -> Result<Option<String>, String> {
        let data_root = app_data_root("WorkBuddy")?;
        let secret = read_workbuddy_secret_storage_value(
            "tencent-cloud.coding-copilot",
            "planning-genie.new.accessTokencn",
            Some(data_root.to_string_lossy().as_ref()),
        )?;
        let Some(secret) = secret else {
            return Ok(None);
        };
        Self::match_codebuddy_like_current_account(db, "workbuddy", &secret)
    }

    fn match_codebuddy_like_current_account(
        db: &Database,
        platform: &str,
        secret: &str,
    ) -> Result<Option<String>, String> {
        let parsed_json = serde_json::from_str::<Value>(secret).ok();
        let token_candidate = parsed_json
            .as_ref()
            .and_then(|value| pick_string(value, &["token", "access_token", "accessToken"]))
            .or_else(|| {
                let raw = secret.trim();
                if raw.is_empty() { None } else { Some(raw.to_string()) }
            });
        let Some(raw_token) = token_candidate else {
            return Ok(None);
        };
        let (uid, token) = if let Some((prefix, suffix)) = raw_token.split_once('+') {
            let uid = normalize_optional_value(Some(prefix));
            let token = normalize_optional_value(Some(suffix));
            (uid, token)
        } else {
            (None, normalize_optional_value(Some(raw_token.as_str())))
        };
        let email = parsed_json
            .as_ref()
            .and_then(|value| pick_string(value, &["email"]))
            .or_else(|| parsed_json.as_ref().and_then(|value| pick_string(value, &["nickname", "name"])));

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;
        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case(platform)),
            email.as_deref(),
            token.as_deref(),
            None,
            uid.as_deref(),
        ))
    }

    fn get_current_qoder_account_id(db: &Database) -> Result<Option<String>, String> {
        let db_path = get_default_qoder_state_db_path()?;
        if !db_path.exists() {
            return Ok(None);
        }

        let user_info = read_qoder_secret_storage_value_by_db_path(
            db_path.as_path(),
            "secret://aicoding.auth.userInfo",
        )?
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());

        let email = user_info
            .as_ref()
            .and_then(|value| pick_string(value, &["email", "userEmail"]));
        let user_id = user_info
            .as_ref()
            .and_then(|value| pick_string(value, &["id", "userId", "user_id", "uid"]));

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case("qoder")),
            email.as_deref(),
            None,
            None,
            user_id.as_deref(),
        ))
    }

    fn get_current_trae_account_id(db: &Database) -> Result<Option<String>, String> {
        let storage_path = get_default_trae_storage_path()?;
        if !storage_path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&storage_path)
            .map_err(|e| format!("读取 Trae storage.json 失败: {}", e))?;
        let storage_root = serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("解析 Trae storage.json 失败: {}", e))?;

        let email = pick_string(
            &storage_root,
            &[
                "email",
                "userEmail",
                "preferred_username",
                "username",
            ],
        );
        let user_id = pick_string(&storage_root, &["userId", "user_id", "sub", "uid"]);

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case("trae")),
            email.as_deref(),
            None,
            None,
            user_id.as_deref(),
        ))
    }

    fn get_current_zed_account_id(db: &Database) -> Result<Option<String>, String> {
        let credentials = read_local_zed_credentials()?;
        let Some((user_id, access_token)) = credentials else {
            return Ok(None);
        };

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts.into_iter().filter(|item| item.origin_platform.eq_ignore_ascii_case("zed")),
            None,
            Some(access_token.as_str()),
            None,
            Some(user_id.as_str()),
        ))
    }
}

fn parse_meta(raw: Option<&str>) -> Value {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null)
}

fn normalize_optional_string(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_optional_value(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn ide_state_db_path(app_name: &str) -> Result<std::path::PathBuf, String> {
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

fn get_default_qoder_state_db_path() -> Result<PathBuf, String> {
    Ok(app_data_root("Qoder")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb"))
}

fn get_default_trae_storage_path() -> Result<PathBuf, String> {
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
fn read_local_zed_credentials() -> Result<Option<(String, String)>, String> {
    let meta_output = security_command_output(&["find-internet-password", "-s", "https://zed.dev"])?;
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
fn read_local_zed_credentials() -> Result<Option<(String, String)>, String> {
    Ok(None)
}

fn read_db_string(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM ItemTable WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .ok()
    .and_then(|value| normalize_optional_value(Some(value.as_str())))
}

fn read_local_kiro_auth_token_json() -> Result<Option<Value>, String> {
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
    let parsed =
        serde_json::from_str::<Value>(&raw).map_err(|e| format!("解析 Kiro 本地授权文件失败: {}", e))?;
    Ok(Some(parsed))
}

fn read_local_kiro_profile_json() -> Result<Option<Value>, String> {
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
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("读取 Kiro profile.json 失败: {}", e))?;
    let parsed = serde_json::from_str::<Value>(&raw).map_err(|e| format!("解析 Kiro profile.json 失败: {}", e))?;
    Ok(Some(parsed))
}

fn pick_string(value: &Value, keys: &[&str]) -> Option<String> {
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

fn find_matching_ide_account_id(
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

fn decode_jwt_claim(token: &str, claim: &str) -> Option<String> {
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
    let payload = base64::engine::general_purpose::STANDARD.decode(padded).ok()?;
    let value: Value = serde_json::from_slice(&payload).ok()?;
    value.get(claim)?.as_str().map(|value| value.to_string())
}
