use base64::Engine;
use rusqlite::OptionalExtension;
/// IDE 本地账号扫描服务
///
/// 支持：
/// 1. 自动扫描本机常见 IDE 存储路径，提取 OAuth Token
/// 2. 从用户指定的 .vscdb 文件提取
/// 3. 旧版 v1 格式账号迁移
/// 4. 从导出 JSON / 平台本地 JSON / state.vscdb 等文件导入
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use crate::services::ide_injector::{
    read_codebuddy_cn_secret_storage_value, read_codebuddy_secret_storage_value,
    read_qoder_secret_storage_value_by_db_path, read_workbuddy_secret_storage_value,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedIdeAccount {
    pub email: String,
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub origin_platform: String,
    pub source_path: String,
    pub meta_json: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImportFailure {
    pub source_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImportScanResult {
    pub accounts: Vec<ScannedIdeAccount>,
    pub failures: Vec<FileImportFailure>,
}

pub struct IdeScanner;

impl IdeScanner {
    /// 自动扫描本机所有已知 IDE 存储路径，提取账号
    pub fn scan_ide_accounts_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let candidates = collect_all_ide_state_db_candidates();
        let mut all: Vec<ScannedIdeAccount> = Vec::new();

        for (path, platform) in &candidates {
            if !path.exists() {
                continue;
            }
            match extract_accounts_from_vscdb(path, platform) {
                Ok(accounts) => all.extend(accounts),
                Err(e) => eprintln!("[IdeScanner] 跳过 {:?}: {}", path, e),
            }
        }

        if all.is_empty() {
            Err("在本机常见路径下未找到任何 IDE 账号数据。请确认 IDE 已安装并曾登录。".to_string())
        } else {
            Ok(all)
        }
    }

    /// 从用户指定的 .vscdb / .db 文件提取账号
    pub fn import_from_custom_db(path: String) -> Result<Vec<ScannedIdeAccount>, String> {
        let path = PathBuf::from(&path);
        if !path.exists() {
            return Err(format!("文件不存在: {:?}", path));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let platform = match ext.as_str() {
            "vscdb" => "vscode",
            _ => "generic_ide",
        };
        extract_accounts_from_vscdb(&path, platform)
    }

    /// 旧版 v1 账号迁移
    pub fn import_v1_accounts() -> Result<Vec<ScannedIdeAccount>, String> {
        let paths = get_v1_migration_paths();
        let mut results = Vec::new();

        for path in &paths {
            if !path.exists() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(content) => match parse_v1_json(&content, path) {
                    Ok(accounts) => results.extend(accounts),
                    Err(e) => eprintln!("[IdeScanner] v1 解析失败 {:?}: {}", path, e),
                },
                Err(e) => eprintln!("[IdeScanner] v1 读取失败 {:?}: {}", path, e),
            }
        }

        if results.is_empty() {
            Err("未找到旧版 v1 格式账号数据".to_string())
        } else {
            Ok(results)
        }
    }

    pub fn import_from_files(paths: Vec<String>) -> Result<FileImportScanResult, String> {
        if paths.is_empty() {
            return Err("请选择至少一个导入文件".to_string());
        }

        let mut accounts = Vec::new();
        let mut failures = Vec::new();

        for raw_path in paths {
            let path = PathBuf::from(&raw_path);
            if !path.exists() {
                failures.push(FileImportFailure {
                    source_path: raw_path,
                    reason: "文件不存在".to_string(),
                });
                continue;
            }

            match extract_accounts_from_import_file(&path) {
                Ok(mut parsed) => accounts.append(&mut parsed),
                Err(reason) => failures.push(FileImportFailure {
                    source_path: path.to_string_lossy().to_string(),
                    reason,
                }),
            }
        }

        dedup_scanned_accounts(&mut accounts);

        if accounts.is_empty() && failures.is_empty() {
            return Err("未从所选文件中解析到可导入的账号数据".to_string());
        }

        Ok(FileImportScanResult { accounts, failures })
    }

    /// 直接读取本地 Gemini CLI 登录态（~/.gemini/oauth_creds.json 等）
    pub fn import_gemini_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_gemini_local_account()?;
        Ok(vec![account])
    }

    pub fn import_codex_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_codex_local_account()?;
        Ok(vec![account])
    }

    pub fn import_kiro_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_kiro_local_account()?;
        Ok(vec![account])
    }

    pub fn import_cursor_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_cursor_local_account()?;
        Ok(vec![account])
    }

    pub fn import_windsurf_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_windsurf_local_account()?;
        Ok(vec![account])
    }

    pub fn import_codebuddy_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_codebuddy_local_account()?;
        Ok(vec![account])
    }

    pub fn import_codebuddy_cn_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_codebuddy_cn_local_account()?;
        Ok(vec![account])
    }

    pub fn import_workbuddy_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_workbuddy_local_account()?;
        Ok(vec![account])
    }

    pub fn import_zed_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_zed_local_account()?;
        Ok(vec![account])
    }

    pub fn import_qoder_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_qoder_local_account()?;
        Ok(vec![account])
    }

    pub fn import_trae_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let account = extract_trae_local_account()?;
        Ok(vec![account])
    }
}

fn dedup_scanned_accounts(accounts: &mut Vec<ScannedIdeAccount>) {
    let mut seen = HashSet::new();
    accounts.retain(|item| {
        let key = format!(
            "{}|{}|{}|{}|{}",
            item.origin_platform,
            item.email,
            item.refresh_token.as_deref().unwrap_or(""),
            item.access_token.as_deref().unwrap_or(""),
            item.meta_json.as_deref().unwrap_or("")
        );
        seen.insert(key)
    });
}

fn extract_accounts_from_import_file(path: &Path) -> Result<Vec<ScannedIdeAccount>, String> {
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
        return IdeScanner::import_from_custom_db(path.to_string_lossy().to_string());
    }

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("读取导入文件失败: {}", e))?;
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
            .and_then(|value| value.get("active").and_then(|item| item.as_str()).map(str::to_string))
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
            .or_else(|| token_obj.and_then(|token| token.get("refresh_token")).and_then(|v| v.as_str()))
            .or_else(|| token_obj.and_then(|token| token.get("refreshToken")).and_then(|v| v.as_str()))
            .map(str::to_string);
        let access_token = obj
            .get("access_token")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("accessToken").and_then(|v| v.as_str()))
            .or_else(|| obj.get("token").and_then(|value| value.as_str()))
            .or_else(|| token_obj.and_then(|token| token.get("access_token")).and_then(|v| v.as_str()))
            .or_else(|| token_obj.and_then(|token| token.get("accessToken")).and_then(|v| v.as_str()))
            .or_else(|| token_obj.and_then(|token| token.get("token")).and_then(|v| v.as_str()))
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
            .or_else(|| obj.get("account").and_then(|value| value.get("email")).and_then(|v| v.as_str()))
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
        Some(serde_json::Value::String(raw)) if !raw.trim().is_empty() => Some(raw.trim().to_string()),
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

// ── vscdb 解析（SQLite）────────────────────────────────────────────────────────

fn extract_accounts_from_vscdb(
    path: &Path,
    platform: &str,
) -> Result<Vec<ScannedIdeAccount>, String> {
    // 由于 vscdb 是 SQLite 数据库，我们使用 rusqlite 提取其中的 auth 数据
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
        // 尝试解析 JSON 值
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&value) {
            if let Some(accounts_from_json) = parse_auth_json(&json, platform, path) {
                accounts.extend(accounts_from_json);
            }
        } else {
            // 非 JSON 值，直接检测 token 特征
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

    // 去重（按 refresh_token 或 access_token）
    accounts.sort_by(|a, b| {
        a.refresh_token
            .cmp(&b.refresh_token)
            .then_with(|| a.access_token.cmp(&b.access_token))
    });
    accounts
        .dedup_by(|a, b| a.refresh_token == b.refresh_token && a.access_token == b.access_token);

    Ok(accounts)
}

fn parse_auth_json(
    json: &serde_json::Value,
    platform: &str,
    path: &Path,
) -> Option<Vec<ScannedIdeAccount>> {
    let mut results = Vec::new();
    let source_path = path.to_string_lossy().to_string();

    // 格式1: 数组 [{account: {email, ...}, refreshToken: ...}, ...]
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

    // 格式2: 对象 {refreshToken: ..., email: ...}
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
                    label: obj.get("label").and_then(|v| v.as_str()).map(|s| s.to_string()),
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
    let mut i = 0;

    // 提取 1// 开头的 refresh token（Claude / Antigravity 格式）
    while i + 3 < bytes.len() {
        if bytes[i] == b'1' && bytes[i + 1] == b'/' && bytes[i + 2] == b'/' {
            let start = i;
            let mut end = i + 3;
            while end < bytes.len() && is_token_char(bytes[end]) {
                end += 1;
            }
            if end - start >= 20 {
                tokens.push(text[start..end].to_string());
            }
            i = end;
        } else {
            i += 1;
        }
    }

    tokens.sort();
    tokens.dedup();
    tokens
}

fn is_token_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.'
}

// ── 路径枚举 ─────────────────────────────────────────────────────────────────

fn collect_all_ide_state_db_candidates() -> Vec<(PathBuf, String)> {
    let mut candidates: Vec<(PathBuf, String)> = Vec::new();

    // IDE 目录名 → 平台标识符
    let ide_dirs: &[(&str, &str)] = &[
        ("Code", "vscode"),
        ("Code - Insiders", "vscode"),
        ("VSCodium", "vscode"),
        ("Cursor", "cursor"),
        ("Cursor - Nightly", "cursor"),
        ("Windsurf", "windsurf"),
        ("Kiro", "kiro"),
        ("Trae", "trae"),
        ("Qoder", "qoder"),
    ];

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = PathBuf::from(&appdata);
            for (dir, platform) in ide_dirs {
                let db = base
                    .join(dir)
                    .join("User")
                    .join("globalStorage")
                    .join("state.vscdb");
                candidates.push((db, platform.to_string()));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let base = home.join("Library").join("Application Support");
            for (dir, platform) in ide_dirs {
                let db = base
                    .join(dir)
                    .join("User")
                    .join("globalStorage")
                    .join("state.vscdb");
                candidates.push((db, platform.to_string()));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let base = home.join(".config");
            for (dir, platform) in ide_dirs {
                let db = base
                    .join(dir)
                    .join("User")
                    .join("globalStorage")
                    .join("state.vscdb");
                candidates.push((db, platform.to_string()));
            }
        }
    }

    candidates
}

fn extract_gemini_local_account() -> Result<ScannedIdeAccount, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let gemini_dir = home.join(".gemini");
    let oauth_path = gemini_dir.join("oauth_creds.json");
    let accounts_path = gemini_dir.join("google_accounts.json");

    if !oauth_path.exists() {
        return Err(format!(
            "未找到本地 Gemini 登录文件: {}",
            oauth_path.to_string_lossy()
        ));
    }

    let oauth_raw = std::fs::read_to_string(&oauth_path)
        .map_err(|e| format!("读取 Gemini oauth_creds.json 失败: {}", e))?;
    let oauth_json: serde_json::Value = serde_json::from_str(&oauth_raw)
        .map_err(|e| format!("解析 Gemini oauth_creds.json 失败: {}", e))?;

    let access_token = oauth_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let refresh_token = oauth_json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let id_token = oauth_json
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if access_token.is_none() && refresh_token.is_none() {
        return Err("本地 Gemini 登录文件缺少 access_token / refresh_token".to_string());
    }

    let email_from_accounts = if accounts_path.exists() {
        let accounts_raw = std::fs::read_to_string(&accounts_path)
            .map_err(|e| format!("读取 Gemini google_accounts.json 失败: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&accounts_raw)
            .ok()
            .and_then(|json| {
                json.get("active")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    } else {
        None
    };

    let email = email_from_accounts
        .or_else(|| id_token.as_deref().and_then(parse_email_from_jwt))
        .unwrap_or_else(|| "unknown@gmail.com".to_string());

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "gemini".to_string(),
        source_path: oauth_path.to_string_lossy().to_string(),
        meta_json: None,
        label: None,
    })
}

fn extract_codex_local_account() -> Result<ScannedIdeAccount, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let auth_path = home.join(".codex").join("auth.json");
    if !auth_path.exists() {
        return Err(format!(
            "未找到本地 Codex 登录文件: {}",
            auth_path.to_string_lossy()
        ));
    }

    let raw = std::fs::read_to_string(&auth_path)
        .map_err(|e| format!("读取 Codex auth.json 失败: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("解析 Codex auth.json 失败: {}", e))?;

    let tokens = json.get("tokens").cloned().unwrap_or(serde_json::json!({}));
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
        return Err("本地 Codex auth.json 缺少 access_token / OPENAI_API_KEY".to_string());
    }

    let email = id_token
        .as_deref()
        .and_then(parse_email_from_jwt)
        .unwrap_or_else(|| "unknown@openai.local".to_string());

    let auth_mode = if openai_api_key
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
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

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "codex".to_string(),
        source_path: auth_path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: None,
    })
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

fn read_state_db_string(db_path: &Path, key: &str) -> Option<String> {
    let conn = rusqlite::Connection::open(db_path).ok()?;
    conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1 LIMIT 1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn extract_cursor_local_account() -> Result<ScannedIdeAccount, String> {
    let db_path = app_data_root("Cursor")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if !db_path.exists() {
        return Err(format!("未找到本地 Cursor 登录数据库: {}", db_path.display()));
    }

    let access_token = read_state_db_string(&db_path, "cursorAuth/accessToken")
        .or_else(|| read_state_db_string(&db_path, "cursor.accessToken"));
    let refresh_token = read_state_db_string(&db_path, "cursorAuth/refreshToken");
    let email = read_state_db_string(&db_path, "cursorAuth/cachedEmail")
        .or_else(|| read_state_db_string(&db_path, "cursor.email"))
        .unwrap_or_else(|| "unknown@cursor.local".to_string());
    let auth_id = read_state_db_string(&db_path, "cursorAuth/authId");
    let membership_type = read_state_db_string(&db_path, "cursorAuth/stripeMembershipType");
    let subscription_status = read_state_db_string(&db_path, "cursorAuth/stripeSubscriptionStatus");

    if access_token.is_none() && refresh_token.is_none() {
        return Err("本地 Cursor 登录数据库缺少 accessToken / refreshToken".to_string());
    }

    let meta_json = serde_json::json!({
        "auth_id": auth_id,
        "membership_type": membership_type,
        "subscription_status": subscription_status,
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "cursor".to_string(),
        source_path: db_path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: None,
    })
}

fn extract_windsurf_local_account() -> Result<ScannedIdeAccount, String> {
    let db_path = app_data_root("Windsurf")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if !db_path.exists() {
        return Err(format!("未找到本地 Windsurf 登录数据库: {}", db_path.display()));
    }

    let auth_status_raw = read_state_db_string(&db_path, "windsurfAuthStatus")
        .ok_or_else(|| "未读取到本地 Windsurf 登录信息".to_string())?;
    let auth_status_json: serde_json::Value = serde_json::from_str(&auth_status_raw)
        .map_err(|e| format!("解析 Windsurf 登录信息失败: {}", e))?;

    let access_token = pick_string_recursive(&auth_status_json, &["apiKey", "api_key", "accessToken", "access_token"]);
    let refresh_token = pick_string_recursive(&auth_status_json, &["refreshToken", "refresh_token"]);
    let email = pick_string_recursive(&auth_status_json, &["email", "userEmail"])
        .unwrap_or_else(|| "unknown@windsurf.local".to_string());
    let user_id = pick_string_recursive(&auth_status_json, &["id", "userId", "user_id", "uid"]);
    let plan = pick_string_recursive(&auth_status_json, &["plan", "planName", "subscription"]);

    if access_token.is_none() && refresh_token.is_none() {
        return Err("本地 Windsurf 登录信息缺少 apiKey / refreshToken".to_string());
    }

    let meta_json = serde_json::json!({
        "user_id": user_id,
        "plan": plan,
        "windsurf_auth_status_raw": auth_status_json,
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "windsurf".to_string(),
        source_path: db_path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: None,
    })
}

fn extract_qoder_local_account() -> Result<ScannedIdeAccount, String> {
    let db_path = app_data_root("Qoder")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if !db_path.exists() {
        return Err(format!("未找到本地 Qoder 登录数据库: {}", db_path.display()));
    }

    let user_info_raw = read_qoder_secret_storage_value_by_db_path(
        db_path.as_path(),
        "secret://aicoding.auth.userInfo",
    )?
    .ok_or_else(|| "未读取到本地 Qoder userInfo".to_string())?;
    let user_plan_raw = read_qoder_secret_storage_value_by_db_path(
        db_path.as_path(),
        "secret://aicoding.auth.userPlan",
    )?
    .unwrap_or_default();
    let credit_usage_raw = read_qoder_secret_storage_value_by_db_path(
        db_path.as_path(),
        "secret://aicoding.auth.creditUsage",
    )?
    .unwrap_or_default();

    let user_info_json: serde_json::Value =
        serde_json::from_str(&user_info_raw).map_err(|e| format!("解析 Qoder userInfo 失败: {}", e))?;
    let user_plan_json = serde_json::from_str::<serde_json::Value>(&user_plan_raw).ok();
    let credit_usage_json = serde_json::from_str::<serde_json::Value>(&credit_usage_raw).ok();

    let email = pick_string_recursive(&user_info_json, &["email", "userEmail"])
        .unwrap_or_else(|| "unknown@qoder.local".to_string());
    let user_id = pick_string_recursive(&user_info_json, &["id", "userId", "user_id", "uid"]);
    let synthetic_access = user_id
        .clone()
        .or_else(|| Some(email.clone()))
        .map(|value| format!("qoder-local:{}", value.replace('@', "_")))
        .unwrap_or_else(|| "qoder-local:unknown".to_string());

    let meta_json = serde_json::json!({
        "user_id": user_id,
        "auth_user_info_raw": user_info_json,
        "auth_user_plan_raw": user_plan_json,
        "auth_credit_usage_raw": credit_usage_json,
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email,
        refresh_token: None,
        access_token: Some(synthetic_access),
        origin_platform: "qoder".to_string(),
        source_path: db_path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: pick_string_recursive(&user_info_json, &["name", "displayName", "nickname"]),
    })
}

fn extract_kiro_local_account() -> Result<ScannedIdeAccount, String> {
    let auth_path = dirs::home_dir()
        .ok_or("无法获取用户主目录".to_string())?
        .join(".aws")
        .join("sso")
        .join("cache")
        .join("kiro-auth-token.json");
    if !auth_path.exists() {
        return Err(format!("未找到本地 Kiro 授权文件: {}", auth_path.display()));
    }

    let profile_path = app_data_root("Kiro")?
        .join("User")
        .join("globalStorage")
        .join("kiro.kiroagent")
        .join("profile.json");

    let auth_raw = std::fs::read_to_string(&auth_path)
        .map_err(|e| format!("读取 Kiro 授权文件失败: {}", e))?;
    let auth_json: serde_json::Value =
        serde_json::from_str(&auth_raw).map_err(|e| format!("解析 Kiro 授权文件失败: {}", e))?;
    let profile_json = if profile_path.exists() {
        std::fs::read_to_string(&profile_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    } else {
        None
    };

    let email = profile_json
        .as_ref()
        .and_then(|value| pick_string_recursive(value, &["email", "userEmail"]))
        .or_else(|| {
            pick_string_recursive(
                &auth_json,
                &["email", "upn", "preferred_username"],
            )
        })
        .unwrap_or_else(|| "unknown@kiro.local".to_string());
    let user_id = profile_json
        .as_ref()
        .and_then(|value| pick_string_recursive(value, &["userId", "user_id", "sub", "accountId"]))
        .or_else(|| pick_string_recursive(&auth_json, &["userId", "user_id", "sub"]));
    let refresh_token = pick_string_recursive(
        &auth_json,
        &["refreshToken", "refresh_token", "refreshTokenJwt"],
    );
    let access_token = pick_string_recursive(
        &auth_json,
        &[
            "accessToken",
            "access_token",
            "token",
            "idToken",
            "id_token",
            "accessTokenJwt",
        ],
    );

    if refresh_token.is_none() && access_token.is_none() {
        return Err("本地 Kiro 授权文件缺少 refresh_token / access_token".to_string());
    }

    let meta_json = serde_json::json!({
        "user_id": user_id,
        "kiro_auth_token_raw": auth_json,
        "kiro_profile_raw": profile_json,
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token,
        origin_platform: "kiro".to_string(),
        source_path: auth_path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: profile_json
            .as_ref()
            .and_then(|value| pick_string_recursive(value, &["name", "displayName", "nickname"])),
    })
}

fn parse_codebuddy_secret(secret: &str, platform: &str, source_path: String) -> Result<ScannedIdeAccount, String> {
    let parsed_json = serde_json::from_str::<serde_json::Value>(secret).ok();
    let token_candidate = parsed_json
        .as_ref()
        .and_then(|value| pick_string_recursive(value, &["token", "access_token", "accessToken"]))
        .or_else(|| {
            let raw = secret.trim();
            if raw.is_empty() { None } else { Some(raw.to_string()) }
        })
        .ok_or_else(|| format!("本地 {} 登录信息解析失败：未找到 access token", platform))?;

    let (uid_from_token, normalized_token) = if let Some((prefix, suffix)) = token_candidate.split_once('+') {
        let uid = prefix.trim();
        let token = suffix.trim();
        if token.is_empty() {
            return Err(format!("本地 {} 登录信息解析失败：access token 无效", platform));
        }
        (
            if uid.is_empty() { None } else { Some(uid.to_string()) },
            token.to_string(),
        )
    } else {
        (None, token_candidate.trim().to_string())
    };

    if normalized_token.is_empty() {
        return Err(format!("本地 {} 登录信息解析失败：access token 为空", platform));
    }

    let root_obj = parsed_json.as_ref().and_then(|v| v.as_object());
    let account_obj = root_obj.and_then(|obj| obj.get("account").and_then(|v| v.as_object()));
    let auth_obj = root_obj.and_then(|obj| obj.get("auth").and_then(|v| v.as_object()));

    let uid = root_obj
        .and_then(|obj| json_object_string_field(obj, &["uid"]))
        .or_else(|| account_obj.and_then(|obj| json_object_string_field(obj, &["uid", "id"])))
        .or(uid_from_token);
    let nickname = root_obj
        .and_then(|obj| json_object_string_field(obj, &["nickname", "name"]))
        .or_else(|| account_obj.and_then(|obj| json_object_string_field(obj, &["nickname", "label"])));
    let email = root_obj
        .and_then(|obj| json_object_string_field(obj, &["email"]))
        .or_else(|| account_obj.and_then(|obj| json_object_string_field(obj, &["email"])))
        .or_else(|| auth_obj.and_then(|obj| json_object_string_field(obj, &["email"])))
        .or_else(|| nickname.clone())
        .or_else(|| uid.clone())
        .unwrap_or_else(|| format!("unknown@{}.local", platform.to_lowercase()));

    let refresh_token = root_obj
        .and_then(|obj| json_object_string_field(obj, &["refreshToken", "refresh_token"]))
        .or_else(|| auth_obj.and_then(|obj| json_object_string_field(obj, &["refreshToken", "refresh_token"])));

    let meta_json = serde_json::json!({
        "uid": uid,
        "nickname": nickname,
        "auth_raw": parsed_json.clone(),
        "profile_raw": account_obj.map(|obj| serde_json::Value::Object(obj.clone())),
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token: Some(normalized_token),
        origin_platform: platform.to_string(),
        source_path,
        meta_json: Some(meta_json),
        label: nickname,
    })
}

fn json_object_string_field(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        let value = obj
            .get(*key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        if let Some(found) = value {
            return Some(found.to_string());
        }
    }
    None
}

fn extract_codebuddy_local_account() -> Result<ScannedIdeAccount, String> {
    let data_root = app_data_root("CodeBuddy")?;
    let source_path = data_root
        .join("User")
        .join("globalStorage")
        .join("state.vscdb")
        .to_string_lossy()
        .to_string();
    let secret = read_codebuddy_secret_storage_value(
        "tencent-cloud.coding-copilot",
        "planning-genie.new.accessToken",
        Some(data_root.to_string_lossy().as_ref()),
    )?
    .ok_or_else(|| "未读取到本地 CodeBuddy 登录信息".to_string())?;
    parse_codebuddy_secret(&secret, "codebuddy", source_path)
}

fn extract_codebuddy_cn_local_account() -> Result<ScannedIdeAccount, String> {
    let data_root = app_data_root("CodeBuddy CN")?;
    let source_path = data_root
        .join("User")
        .join("globalStorage")
        .join("state.vscdb")
        .to_string_lossy()
        .to_string();
    let secret = read_codebuddy_cn_secret_storage_value(
        "tencent-cloud.coding-copilot",
        "planning-genie.new.accessToken",
        Some(data_root.to_string_lossy().as_ref()),
    )?
    .ok_or_else(|| "未读取到本地 CodeBuddy CN 登录信息".to_string())?;
    parse_codebuddy_secret(&secret, "codebuddy_cn", source_path)
}

fn extract_workbuddy_local_account() -> Result<ScannedIdeAccount, String> {
    let data_root = app_data_root("WorkBuddy")?;
    let source_path = data_root
        .join("User")
        .join("globalStorage")
        .join("state.vscdb")
        .to_string_lossy()
        .to_string();
    let secret = read_workbuddy_secret_storage_value(
        "tencent-cloud.coding-copilot",
        "planning-genie.new.accessTokencn",
        Some(data_root.to_string_lossy().as_ref()),
    )?
    .ok_or_else(|| "未读取到本地 WorkBuddy 登录信息".to_string())?;
    parse_codebuddy_secret(&secret, "workbuddy", source_path)
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
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn extract_zed_local_account() -> Result<ScannedIdeAccount, String> {
    let meta_output = security_command_output(&["find-internet-password", "-s", "https://zed.dev"])?;
    if !meta_output.status.success() {
        let stderr = String::from_utf8_lossy(&meta_output.stderr);
        if stderr.contains("could not be found") {
            return Err("未在本机 Zed 客户端登录态中找到可导入的账号信息".to_string());
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
        return Err("Zed Keychain access_token 为空".to_string());
    }

    let meta_json = serde_json::json!({
        "user_id": user_id,
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email: format!("{}@zed.local", user_id),
        refresh_token: None,
        access_token: Some(access_token),
        origin_platform: "zed".to_string(),
        source_path: "keychain://zed.dev".to_string(),
        meta_json: Some(meta_json),
        label: Some(user_id),
    })
}

#[cfg(not(target_os = "macos"))]
fn extract_zed_local_account() -> Result<ScannedIdeAccount, String> {
    Err("Zed 本地导入当前仅支持 macOS".to_string())
}

fn extract_trae_local_account() -> Result<ScannedIdeAccount, String> {
    let storage_path = app_data_root("Trae")?
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    if !storage_path.exists() {
        return Err(format!("未找到本地 Trae storage.json: {}", storage_path.display()));
    }

    let raw = std::fs::read_to_string(&storage_path)
        .map_err(|e| format!("读取 Trae storage.json 失败: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("解析 Trae storage.json 失败: {}", e))?;

    let email = pick_string_recursive(
        &json,
        &["email", "userEmail", "preferred_username", "username"],
    )
    .unwrap_or_else(|| "unknown@trae.local".to_string());
    let user_id = pick_string_recursive(&json, &["userId", "user_id", "sub", "uid"]);
    let access_token = pick_string_recursive(
        &json,
        &["token", "accessToken", "access_token"],
    );
    let refresh_token = pick_string_recursive(
        &json,
        &["refreshToken", "refresh_token"],
    );
    let synthetic_access = access_token.clone().unwrap_or_else(|| {
        format!(
            "trae-local:{}",
            user_id
                .clone()
                .unwrap_or_else(|| email.clone())
                .replace('@', "_")
        )
    });

    let meta_json = serde_json::json!({
        "user_id": user_id,
        "trae_storage_raw": json,
    })
    .to_string();

    Ok(ScannedIdeAccount {
        email,
        refresh_token,
        access_token: Some(synthetic_access),
        origin_platform: "trae".to_string(),
        source_path: storage_path.to_string_lossy().to_string(),
        meta_json: Some(meta_json),
        label: pick_string_recursive(&json, &["nickname", "username", "displayName", "name"]),
    })
}

fn pick_string_recursive(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = find_string_recursive(value, key) {
            return Some(found);
        }
    }
    None
}

fn find_string_recursive(value: &serde_json::Value, key: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(found) = map.get(key).and_then(|item| item.as_str()) {
                let trimmed = found.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            map.values()
                .find_map(|item| find_string_recursive(item, key))
        }
        serde_json::Value::Array(items) => items
            .iter()
            .find_map(|item| find_string_recursive(item, key)),
        _ => None,
    }
}

fn parse_email_from_jwt(token: &str) -> Option<String> {
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
    let value: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    value.get("email")?.as_str().map(|s| s.to_string())
}

// ── v1 迁移路径 ───────────────────────────────────────────────────────────────

fn get_v1_migration_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".ai-singularity").join("accounts.json"));
        paths.push(home.join(".antigravity").join("accounts.json"));
        paths.push(home.join(".cursor-pool").join("accounts.json"));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("accounts.json"));
            paths.push(dir.join("data").join("accounts.json"));
        }
    }

    paths
}

// ── v1 JSON 解析 ──────────────────────────────────────────────────────────────

fn parse_v1_json(content: &str, path: &Path) -> Result<Vec<ScannedIdeAccount>, String> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("JSON 解析失败: {}", e))?;

    let arr = match &json {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(_) => vec![json.clone()],
        _ => return Err("不支持的 JSON 格式".to_string()),
    };

    let accounts = arr
        .into_iter()
        .filter_map(|item| {
            let refresh_token = item["refresh_token"]
                .as_str()
                .or_else(|| item["refreshToken"].as_str())
                .map(|s| s.to_string());
            let access_token = item["access_token"]
                .as_str()
                .or_else(|| item["accessToken"].as_str())
                .or_else(|| item["token"].as_str())
                .map(|s| s.to_string());
            let email = item["email"].as_str().unwrap_or("").to_string();
            let platform = item["origin_platform"]
                .as_str()
                .or_else(|| item["platform"].as_str())
                .unwrap_or("generic_ide")
                .to_string();

            if refresh_token.is_none() && access_token.is_none() {
                return None;
            }

            Some(ScannedIdeAccount {
                email,
                refresh_token,
                access_token,
                origin_platform: platform,
                source_path: path.to_string_lossy().to_string(),
                meta_json: None,
                label: item["label"]
                    .as_str()
                    .or_else(|| item["name"].as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect();

    Ok(accounts)
}
