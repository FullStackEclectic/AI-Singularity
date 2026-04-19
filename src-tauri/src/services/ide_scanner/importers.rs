use super::{ScannedIdeAccount, dedup_scanned_accounts, parse_email_from_jwt};
use std::path::Path;

pub(super) fn extract_accounts_from_import_file(
    path: &Path,
) -> Result<Vec<ScannedIdeAccount>, String> {
    let lower_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "vscdb" | "db") {
        let platform = if ext == "vscdb" {
            "vscode"
        } else {
            "generic_ide"
        };
        return extract_accounts_from_vscdb(path, platform);
    }

    let content = std::fs::read_to_string(path).map_err(|e| format!("读取导入文件失败: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析导入文件失败: {}", e))?;

    if lower_name == "auth.json" {
        if let Some(account) = parse_codex_auth_json(&json, path)? {
            return Ok(vec![account]);
        }
    }

    if lower_name == "oauth_creds.json" {
        if let Some(account) = parse_gemini_oauth_file(&json, path)? {
            return Ok(vec![account]);
        }
    }

    let accounts = parse_generic_import_json(&json, path)?;
    if accounts.is_empty() {
        Err("文件格式暂不支持，未识别到可导入账号".to_string())
    } else {
        Ok(accounts)
    }
}

fn parse_codex_auth_json(
    json: &serde_json::Value,
    path: &Path,
) -> Result<Option<ScannedIdeAccount>, String> {
    let Some(obj) = json.as_object() else {
        return Ok(None);
    };

    if !obj.contains_key("tokens") && !obj.contains_key("OPENAI_API_KEY") {
        return Ok(None);
    }

    let tokens = json
        .get("tokens")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let access_token = tokens
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let refresh_token = tokens
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let id_token = tokens
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let account_id = tokens
        .get("account_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let openai_api_key = json
        .get("OPENAI_API_KEY")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let api_base_url = json
        .get("base_url")
        .or_else(|| json.get("openai_base_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if access_token.is_none() && openai_api_key.is_none() {
        return Err("Codex auth.json 缺少 access_token / OPENAI_API_KEY".to_string());
    }

    let email = id_token
        .as_deref()
        .and_then(parse_email_from_jwt)
        .unwrap_or_else(|| "unknown@openai.local".to_string());
    let auth_mode = if openai_api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "apikey"
    } else {
        "oauth"
    };

    let meta_json = serde_json::json!({
        "auth_mode": auth_mode,
        "id_token": id_token,
        "account_id": account_id,
        "openai_api_key": openai_api_key,
        "api_base_url": api_base_url,
        "last_refresh": json.get("last_refresh").cloned().unwrap_or(serde_json::Value::Null),
    })
    .to_string();

    Ok(Some(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "codex".to_string(),
        source_path: path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: None,
    }))
}

fn parse_gemini_oauth_file(
    json: &serde_json::Value,
    path: &Path,
) -> Result<Option<ScannedIdeAccount>, String> {
    let Some(obj) = json.as_object() else {
        return Ok(None);
    };

    if !obj.contains_key("access_token")
        && !obj.contains_key("refresh_token")
        && !obj.contains_key("id_token")
    {
        return Ok(None);
    }

    let access_token = obj
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let refresh_token = obj
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let id_token = obj
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if access_token.is_none() && refresh_token.is_none() {
        return Err("Gemini oauth_creds.json 缺少 access_token / refresh_token".to_string());
    }

    let email_from_accounts = path.parent().and_then(|dir| {
        let accounts_path = dir.join("google_accounts.json");
        if !accounts_path.exists() {
            return None;
        }
        std::fs::read_to_string(accounts_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| {
                value
                    .get("active")
                    .and_then(|item| item.as_str())
                    .map(str::to_string)
            })
    });

    let email = email_from_accounts
        .or_else(|| id_token.as_deref().and_then(parse_email_from_jwt))
        .unwrap_or_else(|| "unknown@gmail.com".to_string());

    Ok(Some(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "gemini".to_string(),
        source_path: path.to_string_lossy().to_string(),
        meta_json: None,
        label: None,
    }))
}

fn parse_generic_import_json(
    json: &serde_json::Value,
    path: &Path,
) -> Result<Vec<ScannedIdeAccount>, String> {
    let fallback_platform = infer_platform_from_path(path);
    let source_path = path.to_string_lossy().to_string();
    let mut results = Vec::new();

    if let Some(parsed) = parse_auth_json(json, &fallback_platform, path) {
        results.extend(parsed);
    }

    let items: Vec<&serde_json::Value> = match json {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        serde_json::Value::Object(obj) => {
            if let Some(accounts) = obj.get("accounts").and_then(|value| value.as_array()) {
                accounts.iter().collect()
            } else if let Some(items) = obj.get("items").and_then(|value| value.as_array()) {
                items.iter().collect()
            } else if let Some(data) = obj.get("data").and_then(|value| value.as_array()) {
                data.iter().collect()
            } else {
                vec![json]
            }
        }
        _ => return Err("JSON 必须是对象或数组".to_string()),
    };

    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let token_obj = obj.get("token").and_then(|value| value.as_object());
        let refresh_token = obj
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("refreshToken").and_then(|v| v.as_str()))
            .or_else(|| {
                token_obj
                    .and_then(|token| token.get("refresh_token"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                token_obj
                    .and_then(|token| token.get("refreshToken"))
                    .and_then(|v| v.as_str())
            })
            .map(str::to_string);
        let access_token = obj
            .get("access_token")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("accessToken").and_then(|v| v.as_str()))
            .or_else(|| obj.get("token").and_then(|value| value.as_str()))
            .or_else(|| {
                token_obj
                    .and_then(|token| token.get("access_token"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                token_obj
                    .and_then(|token| token.get("accessToken"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                token_obj
                    .and_then(|token| token.get("token"))
                    .and_then(|v| v.as_str())
            })
            .map(str::to_string);
        let meta_json = stringify_meta_field(
            obj.get("meta_json")
                .or_else(|| obj.get("metaJson"))
                .or_else(|| obj.get("meta")),
        );
        let meta_value = meta_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
        let meta_obj = meta_value.as_ref().and_then(|value| value.as_object());
        let openai_api_key = meta_obj
            .and_then(|map| map.get("openai_api_key"))
            .and_then(|value| value.as_str())
            .map(str::to_string);

        if refresh_token.is_none() && access_token.is_none() && openai_api_key.is_none() {
            continue;
        }

        let email = obj
            .get("email")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("account_email").and_then(|v| v.as_str()))
            .or_else(|| {
                obj.get("account")
                    .and_then(|value| value.get("email"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or_default()
            .to_string();
        let origin_platform = obj
            .get("origin_platform")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("platform").and_then(|v| v.as_str()))
            .or_else(|| obj.get("provider").and_then(|v| v.as_str()))
            .unwrap_or(&fallback_platform)
            .to_string();
        let label = obj
            .get("label")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("name").and_then(|v| v.as_str()))
            .or_else(|| obj.get("account_name").and_then(|v| v.as_str()))
            .or_else(|| obj.get("displayName").and_then(|v| v.as_str()))
            .or_else(|| obj.get("nickname").and_then(|v| v.as_str()))
            .or_else(|| {
                obj.get("account")
                    .and_then(|value| value.get("label"))
                    .and_then(|v| v.as_str())
            })
            .map(str::to_string);

        results.push(ScannedIdeAccount {
            email,
            refresh_token,
            access_token,
            origin_platform,
            source_path: source_path.clone(),
            meta_json,
            label,
        });
    }

    dedup_scanned_accounts(&mut results);
    Ok(results)
}

fn stringify_meta_field(value: Option<&serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(raw)) if !raw.trim().is_empty() => {
            Some(raw.trim().to_string())
        }
        Some(other) if !other.is_null() => serde_json::to_string(other).ok(),
        _ => None,
    }
}

fn infer_platform_from_path(path: &Path) -> String {
    let joined = path.to_string_lossy().to_ascii_lowercase();
    if joined.contains("codex") {
        "codex".to_string()
    } else if joined.contains("gemini") {
        "gemini".to_string()
    } else if joined.contains("kiro") {
        "kiro".to_string()
    } else if joined.contains("codebuddy cn") || joined.contains("codebuddy_cn") {
        "codebuddy_cn".to_string()
    } else if joined.contains("codebuddy") {
        "codebuddy".to_string()
    } else if joined.contains("workbuddy") {
        "workbuddy".to_string()
    } else if joined.contains("qoder") {
        "qoder".to_string()
    } else if joined.contains("trae") {
        "trae".to_string()
    } else if joined.contains("windsurf") {
        "windsurf".to_string()
    } else if joined.contains("cursor") {
        "cursor".to_string()
    } else if joined.contains("zed") {
        "zed".to_string()
    } else {
        "generic_ide".to_string()
    }
}

pub(super) fn extract_accounts_from_vscdb(
    path: &Path,
    platform: &str,
) -> Result<Vec<ScannedIdeAccount>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM ItemTable WHERE key LIKE '%token%' OR key LIKE '%auth%' OR key LIKE '%session%' OR key LIKE '%secret%'")
        .map_err(|e| format!("查询失败: {}", e))?;

    let mut accounts: Vec<ScannedIdeAccount> = Vec::new();

    let rows = stmt
        .query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })
        .map_err(|e| format!("遍历结果失败: {}", e))?;

    for row in rows.flatten() {
        let (_key, value) = row;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&value) {
            if let Some(accounts_from_json) = parse_auth_json(&json, platform, path) {
                accounts.extend(accounts_from_json);
            }
        } else {
            let tokens = extract_tokens_from_text(&value);
            for token in tokens {
                accounts.push(ScannedIdeAccount {
                    email: "".to_string(),
                    refresh_token: if token.starts_with("1//") {
                        Some(token)
                    } else {
                        None
                    },
                    access_token: None,
                    origin_platform: platform.to_string(),
                    source_path: path.to_string_lossy().to_string(),
                    meta_json: None,
                    label: None,
                });
            }
        }
    }

    accounts.sort_by(|a, b| {
        a.refresh_token
            .cmp(&b.refresh_token)
            .then_with(|| a.access_token.cmp(&b.access_token))
    });
    accounts.dedup_by(|a, b| {
        a.refresh_token == b.refresh_token && a.access_token == b.access_token
    });

    Ok(accounts)
}

fn parse_auth_json(
    json: &serde_json::Value,
    platform: &str,
    path: &Path,
) -> Option<Vec<ScannedIdeAccount>> {
    let mut results = Vec::new();
    let source_path = path.to_string_lossy().to_string();

    if let Some(arr) = json.as_array() {
        for item in arr {
            let email = item["account"]["label"]
                .as_str()
                .or_else(|| item["account"]["email"].as_str())
                .or_else(|| item["email"].as_str())
                .unwrap_or("")
                .to_string();
            let refresh_token = item["refreshToken"]
                .as_str()
                .or_else(|| item["refresh_token"].as_str())
                .map(|s| s.to_string());
            let access_token = item["accessToken"]
                .as_str()
                .or_else(|| item["access_token"].as_str())
                .or_else(|| item["token"].as_str())
                .map(|s| s.to_string());

            if refresh_token.is_some() || access_token.is_some() {
                results.push(ScannedIdeAccount {
                    email,
                    refresh_token,
                    access_token,
                    origin_platform: platform.to_string(),
                    source_path: source_path.clone(),
                    meta_json: None,
                    label: item["account"]["label"]
                        .as_str()
                        .or_else(|| item["name"].as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    if results.is_empty() {
        if let Some(obj) = json.as_object() {
            let refresh_token = obj
                .get("refreshToken")
                .or_else(|| obj.get("refresh_token"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let access_token = obj
                .get("accessToken")
                .or_else(|| obj.get("access_token"))
                .or_else(|| obj.get("token"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let email = obj
                .get("email")
                .or_else(|| obj.get("login"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if refresh_token.is_some() || access_token.is_some() {
                results.push(ScannedIdeAccount {
                    email,
                    refresh_token,
                    access_token,
                    origin_platform: platform.to_string(),
                    source_path,
                    meta_json: None,
                    label: obj
                        .get("label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

fn extract_tokens_from_text(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while index + 3 < bytes.len() {
        if bytes[index] == b'1' && bytes[index + 1] == b'/' && bytes[index + 2] == b'/' {
            let mut end = index + 3;
            while end < bytes.len() && is_token_char(bytes[end]) {
                end += 1;
            }
            let token = &text[index..end];
            if token.len() > 10 {
                tokens.push(token.to_string());
            }
            index = end;
        } else {
            index += 1;
        }
    }

    tokens
}

fn is_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.'
}
