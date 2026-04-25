use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount, OAuthToken};
use crate::services::event_bus::EventBus;
use crate::services::ide_injector::read_antigravity_secret_storage_value;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const EXTENSION_IDS: &[&str] = &[
    "jlcodes.antigravity-cockpit",
    "jlcodes99.antigravity-cockpit",
];
const KEY_MULTI: &str = "antigravity.autoTrigger.credentials";
const KEY_LEGACY: &str = "antigravity.autoTrigger.credential";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionScanResult {
    pub source: String,
    pub extension_id: String,
    pub email: String,
    pub project_id: Option<String>,
    pub has_refresh_token: bool,
}

#[derive(Debug, Deserialize)]
struct ExtensionCredential {
    pub email: Option<String>,
    #[serde(rename = "refreshToken", alias = "refresh_token")]
    pub refresh_token: Option<String>,
    #[serde(rename = "projectId", alias = "project_id")]
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionCredentialsFile {
    accounts: HashMap<String, ExtensionCredential>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionImportProgress {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionImportStats {
    pub scanned: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub details: Vec<String>,
}

pub struct ExtensionImportService;

impl ExtensionImportService {
    pub fn scan() -> Vec<ExtensionScanResult> {
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for ext_id in EXTENSION_IDS {
            for key in [KEY_MULTI, KEY_LEGACY] {
                let parsed = match read_antigravity_secret_storage_value(ext_id, key, None) {
                    Ok(Some(content)) => parse_payload(&content).unwrap_or_default(),
                    _ => continue,
                };
                for (_id, cred) in parsed {
                    let Some(email) = cred.email.as_ref().map(|s| s.trim().to_lowercase()) else {
                        continue;
                    };
                    if email.is_empty() || !email.contains('@') {
                        continue;
                    }
                    let dedup_key = format!("{}::{}", ext_id, email);
                    if !seen.insert(dedup_key) {
                        continue;
                    }
                    let has_refresh = cred
                        .refresh_token
                        .as_ref()
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    results.push(ExtensionScanResult {
                        source: "antigravity".to_string(),
                        extension_id: ext_id.to_string(),
                        email,
                        project_id: cred.project_id.clone(),
                        has_refresh_token: has_refresh,
                    });
                }
            }
        }
        results
    }

    pub fn import_all(db: &Database, app: Option<&AppHandle>) -> ExtensionImportStats {
        let mut stats = ExtensionImportStats {
            scanned: 0,
            imported: 0,
            skipped: 0,
            failed: 0,
            details: Vec::new(),
        };

        let mut credentials: Vec<(String, ExtensionCredential)> = Vec::new();
        for ext_id in EXTENSION_IDS {
            for key in [KEY_MULTI, KEY_LEGACY] {
                if let Ok(Some(content)) = read_antigravity_secret_storage_value(ext_id, key, None)
                {
                    if let Some(parsed) = parse_payload(&content) {
                        for (_id, cred) in parsed {
                            credentials.push((ext_id.to_string(), cred));
                        }
                    }
                }
            }
        }

        stats.scanned = credentials.len();
        let total = credentials.len();
        for (idx, (ext_id, cred)) in credentials.into_iter().enumerate() {
            let email = match cred.email.as_ref().map(|s| s.trim().to_lowercase()) {
                Some(e) if !e.is_empty() && e.contains('@') => e,
                _ => {
                    stats.skipped += 1;
                    continue;
                }
            };
            let refresh_token = cred
                .refresh_token
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let Some(refresh_token) = refresh_token else {
                stats.skipped += 1;
                stats.details.push(format!("{} 缺少 refresh_token", email));
                continue;
            };

            emit_progress(app, "importing", idx, total, Some(&email));

            let now = Utc::now();
            let account = IdeAccount {
                id: Uuid::new_v4().to_string(),
                email: email.clone(),
                origin_platform: "antigravity".to_string(),
                token: OAuthToken {
                    access_token: "requires_refresh".to_string(),
                    refresh_token,
                    expires_in: 0,
                    token_type: "Bearer".to_string(),
                    updated_at: now,
                },
                status: AccountStatus::Active,
                disabled_reason: None,
                is_proxy_disabled: false,
                created_at: now,
                updated_at: now,
                last_used: now,
                device_profile: None,
                quota_json: None,
                project_id: cred.project_id.clone(),
                meta_json: Some(
                    serde_json::json!({
                        "auth_mode": "oauth",
                        "oauth_provider": "antigravity",
                        "import_source": "vscode_extension",
                        "extension_id": ext_id,
                    })
                    .to_string(),
                ),
                label: None,
                tags: vec!["extension".to_string()],
                disabled_at: None,
                fingerprint_id: crate::services::device_fingerprint::DeviceFingerprintService::lookup_for_email(db, &email),
                quota_error_json: None,
            };

            match db.upsert_ide_account(&account) {
                Ok(_) => stats.imported += 1,
                Err(e) => {
                    stats.failed += 1;
                    stats.details.push(format!("{}: 写入失败 {}", email, e));
                }
            }
        }
        emit_progress(app, "done", total, total, None);

        if stats.imported > 0 {
            if let Some(app_handle) = app {
                EventBus::emit_data_changed(
                    app_handle,
                    "ide_accounts",
                    "extension_import",
                    "extension_import.batch",
                );
            }
        }
        stats
    }
}

fn parse_payload(payload: &str) -> Option<HashMap<String, ExtensionCredential>> {
    if let Ok(parsed) = serde_json::from_str::<ExtensionCredentialsFile>(payload) {
        return Some(parsed.accounts);
    }
    if let Ok(single) = serde_json::from_str::<ExtensionCredential>(payload) {
        let mut map = HashMap::new();
        let key = single
            .email
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "__legacy__".to_string());
        map.insert(key, single);
        return Some(map);
    }
    None
}

fn emit_progress(
    app: Option<&AppHandle>,
    phase: &str,
    current: usize,
    total: usize,
    email: Option<&str>,
) {
    let Some(app_handle) = app else {
        return;
    };
    let payload = ExtensionImportProgress {
        phase: phase.to_string(),
        current,
        total,
        email: email.map(|s| s.to_string()),
    };
    let _ = app_handle.emit("accounts:extension-import-progress", payload);
}
