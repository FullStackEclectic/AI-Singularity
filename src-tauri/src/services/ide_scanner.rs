/// IDE 本地账号扫描服务
///
/// 支持：
/// 1. 自动扫描本机常见 IDE 存储路径，提取 OAuth Token
/// 2. 从用户指定的 .vscdb 文件提取
/// 3. 旧版 v1 格式账号迁移
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedIdeAccount {
    pub email:           String,
    pub refresh_token:   Option<String>,
    pub access_token:    Option<String>,
    pub origin_platform: String,
    pub source_path:     String,
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
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let platform = match ext.as_str() {
            "vscdb" => "vscode",
            _       => "generic_ide",
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
                    email:           "".to_string(),
                    refresh_token:   if token.starts_with("1//") { Some(token) } else { None },
                    access_token:    None,
                    origin_platform: platform.to_string(),
                    source_path:     path.to_string_lossy().to_string(),
                });
            }
        }
    }

    // 去重（按 refresh_token 或 access_token）
    accounts.sort_by(|a, b| {
        a.refresh_token.cmp(&b.refresh_token)
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

    // 格式1: 数组 [{account: {email, ...}, refreshToken: ...}, ...]
    if let Some(arr) = json.as_array() {
        for item in arr {
            let email = item["account"]["label"].as_str()
                .or_else(|| item["account"]["email"].as_str())
                .or_else(|| item["email"].as_str())
                .unwrap_or("")
                .to_string();
            let refresh_token = item["refreshToken"].as_str()
                .or_else(|| item["refresh_token"].as_str())
                .map(|s| s.to_string());
            let access_token = item["accessToken"].as_str()
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
                });
            }
        }
    }

    // 格式2: 对象 {refreshToken: ..., email: ...}
    if results.is_empty() {
        if let Some(obj) = json.as_object() {
            let refresh_token = obj.get("refreshToken")
                .or_else(|| obj.get("refresh_token"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let access_token = obj.get("accessToken")
                .or_else(|| obj.get("access_token"))
                .or_else(|| obj.get("token"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let email = obj.get("email")
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
                });
            }
        }
    }

    if results.is_empty() { None } else { Some(results) }
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
        ("Code",            "vscode"),
        ("Code - Insiders", "vscode"),
        ("VSCodium",        "vscode"),
        ("Cursor",          "cursor"),
        ("Cursor - Nightly","cursor"),
        ("Windsurf",        "windsurf"),
        ("Kiro",            "kiro"),
        ("Trae",            "trae"),
        ("Qoder",           "qoder"),
    ];

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = PathBuf::from(&appdata);
            for (dir, platform) in ide_dirs {
                let db = base.join(dir).join("User").join("globalStorage").join("state.vscdb");
                candidates.push((db, platform.to_string()));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let base = home.join("Library").join("Application Support");
            for (dir, platform) in ide_dirs {
                let db = base.join(dir).join("User").join("globalStorage").join("state.vscdb");
                candidates.push((db, platform.to_string()));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let base = home.join(".config");
            for (dir, platform) in ide_dirs {
                let db = base.join(dir).join("User").join("globalStorage").join("state.vscdb");
                candidates.push((db, platform.to_string()));
            }
        }
    }

    candidates
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

fn parse_v1_json(
    content: &str,
    path: &Path,
) -> Result<Vec<ScannedIdeAccount>, String> {
    let json: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    let arr = match &json {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(_) => vec![json.clone()],
        _ => return Err("不支持的 JSON 格式".to_string()),
    };

    let accounts = arr.into_iter().filter_map(|item| {
        let refresh_token = item["refresh_token"].as_str()
            .or_else(|| item["refreshToken"].as_str())
            .map(|s| s.to_string());
        let access_token = item["access_token"].as_str()
            .or_else(|| item["accessToken"].as_str())
            .or_else(|| item["token"].as_str())
            .map(|s| s.to_string());
        let email = item["email"].as_str().unwrap_or("").to_string();
        let platform = item["origin_platform"].as_str()
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
        })
    }).collect();

    Ok(accounts)
}
