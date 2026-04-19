mod importers;
mod local;

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

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
    pub fn scan_ide_accounts_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        let candidates = local::collect_all_ide_state_db_candidates();
        let mut all: Vec<ScannedIdeAccount> = Vec::new();

        for (path, platform) in &candidates {
            if !path.exists() {
                continue;
            }
            match importers::extract_accounts_from_vscdb(path, platform) {
                Ok(accounts) => all.extend(accounts),
                Err(err) => eprintln!("[IdeScanner] 跳过 {:?}: {}", path, err),
            }
        }

        if all.is_empty() {
            Err("在本机常见路径下未找到任何 IDE 账号数据。请确认 IDE 已安装并曾登录。".to_string())
        } else {
            Ok(all)
        }
    }

    pub fn import_from_custom_db(path: String) -> Result<Vec<ScannedIdeAccount>, String> {
        let path = PathBuf::from(&path);
        if !path.exists() {
            return Err(format!("文件不存在: {:?}", path));
        }
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        let platform = match ext.as_str() {
            "vscdb" => "vscode",
            _ => "generic_ide",
        };
        importers::extract_accounts_from_vscdb(&path, platform)
    }

    pub fn import_v1_accounts() -> Result<Vec<ScannedIdeAccount>, String> {
        let paths = local::get_v1_migration_paths();
        let mut results = Vec::new();

        for path in &paths {
            if !path.exists() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(content) => match local::parse_v1_json(&content, path) {
                    Ok(accounts) => results.extend(accounts),
                    Err(err) => eprintln!("[IdeScanner] v1 解析失败 {:?}: {}", path, err),
                },
                Err(err) => eprintln!("[IdeScanner] v1 读取失败 {:?}: {}", path, err),
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

            match importers::extract_accounts_from_import_file(&path) {
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

    pub fn import_gemini_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_gemini_local_account()?])
    }

    pub fn import_codex_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_codex_local_account()?])
    }

    pub fn import_kiro_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_kiro_local_account()?])
    }

    pub fn import_cursor_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_cursor_local_account()?])
    }

    pub fn import_windsurf_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_windsurf_local_account()?])
    }

    pub fn import_codebuddy_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_codebuddy_local_account()?])
    }

    pub fn import_codebuddy_cn_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_codebuddy_cn_local_account()?])
    }

    pub fn import_workbuddy_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_workbuddy_local_account()?])
    }

    pub fn import_zed_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_zed_local_account()?])
    }

    pub fn import_qoder_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_qoder_local_account()?])
    }

    pub fn import_trae_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
        Ok(vec![local::extract_trae_local_account()?])
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
