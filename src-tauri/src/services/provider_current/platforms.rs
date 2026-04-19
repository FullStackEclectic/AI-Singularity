use super::helpers::{
    app_data_root, decode_jwt_claim, find_matching_ide_account_id, get_default_qoder_state_db_path,
    get_default_trae_storage_path, ide_state_db_path, normalize_optional_string,
    normalize_optional_value, parse_meta, pick_string, read_db_string,
    read_local_kiro_auth_token_json, read_local_kiro_profile_json, read_local_zed_credentials,
};
use super::ProviderCurrentService;
use crate::db::Database;
use crate::services::ide_injector::{
    read_codebuddy_cn_secret_storage_value, read_codebuddy_secret_storage_value,
    read_qoder_secret_storage_value_by_db_path, read_workbuddy_secret_storage_value,
};
use serde_json::Value;

impl ProviderCurrentService {
    pub(super) fn get_current_codex_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let auth_path = home.join(".codex").join("auth.json");
        if !auth_path.exists() {
            return Ok(None);
        }

        let raw = std::fs::read_to_string(&auth_path)
            .map_err(|e| format!("读取 Codex auth.json 失败: {}", e))?;
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
                let same_base = normalize_optional_string(
                    meta.get("api_base_url").and_then(|value| value.as_str()),
                ) == base_url.clone();

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
            .or_else(|| {
                access_token
                    .as_deref()
                    .and_then(|token| decode_jwt_claim(token, "email"))
            });

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

    pub(super) fn get_current_gemini_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
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
                    .and_then(|json| {
                        json.get("active")
                            .and_then(|value| value.as_str())
                            .map(|value| value.to_string())
                    })
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

    pub(super) fn get_current_cursor_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
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
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("cursor")),
            email.as_deref(),
            access_token.as_deref(),
            None,
            None,
        ))
    }

    pub(super) fn get_current_windsurf_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
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
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("windsurf")),
            email.as_deref(),
            api_key.as_deref(),
            api_key.as_deref(),
            None,
        ))
    }

    pub(super) fn get_current_kiro_account_id(db: &Database) -> Result<Option<String>, String> {
        let local_auth = read_local_kiro_auth_token_json()?;
        let local_profile = read_local_kiro_profile_json()?;

        let email = local_profile
            .as_ref()
            .and_then(|value| pick_string(value, &["email", "userEmail"]))
            .or_else(|| {
                local_auth
                    .as_ref()
                    .and_then(|value| pick_string(value, &["email", "upn", "preferred_username"]))
            });
        let user_id = local_profile
            .as_ref()
            .and_then(|value| pick_string(value, &["userId", "user_id", "sub", "accountId"]))
            .or_else(|| {
                local_auth
                    .as_ref()
                    .and_then(|value| pick_string(value, &["userId", "user_id", "sub"]))
            });
        let refresh_token = local_auth.as_ref().and_then(|value| {
            pick_string(value, &["refreshToken", "refresh_token", "refreshTokenJwt"])
        });

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("kiro")),
            email.as_deref(),
            None,
            refresh_token.as_deref(),
            user_id.as_deref(),
        ))
    }

    pub(super) fn get_current_codebuddy_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
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

    pub(super) fn get_current_codebuddy_cn_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
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

    pub(super) fn get_current_workbuddy_account_id(
        db: &Database,
    ) -> Result<Option<String>, String> {
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
                if raw.is_empty() {
                    None
                } else {
                    Some(raw.to_string())
                }
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
            .or_else(|| {
                parsed_json
                    .as_ref()
                    .and_then(|value| pick_string(value, &["nickname", "name"]))
            });

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;
        Ok(find_matching_ide_account_id(
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case(platform)),
            email.as_deref(),
            token.as_deref(),
            None,
            uid.as_deref(),
        ))
    }

    pub(super) fn get_current_qoder_account_id(db: &Database) -> Result<Option<String>, String> {
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
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("qoder")),
            email.as_deref(),
            None,
            None,
            user_id.as_deref(),
        ))
    }

    pub(super) fn get_current_trae_account_id(db: &Database) -> Result<Option<String>, String> {
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
            &["email", "userEmail", "preferred_username", "username"],
        );
        let user_id = pick_string(&storage_root, &["userId", "user_id", "sub", "uid"]);

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("trae")),
            email.as_deref(),
            None,
            None,
            user_id.as_deref(),
        ))
    }

    pub(super) fn get_current_zed_account_id(db: &Database) -> Result<Option<String>, String> {
        let credentials = read_local_zed_credentials()?;
        let Some((user_id, access_token)) = credentials else {
            return Ok(None);
        };

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取 IDE 账号失败: {}", e))?;

        Ok(find_matching_ide_account_id(
            accounts
                .into_iter()
                .filter(|item| item.origin_platform.eq_ignore_ascii_case("zed")),
            None,
            Some(access_token.as_str()),
            None,
            Some(user_id.as_str()),
        ))
    }
}
