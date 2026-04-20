use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use crate::services::ide_scanner::IdeScanner;
use chrono::Utc;
use serde_json::Value;

pub struct LocalIdeRefreshService;

impl LocalIdeRefreshService {
    pub fn refresh_vscode_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let account = Self::load_account(db, id)?;
        let scanned = Self::select_scanned_account(
            IdeScanner::import_vscode_from_local()?,
            &account,
            "VS Code",
        )?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_github_copilot_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let account = Self::load_account(db, id)?;
        let scanned = Self::select_scanned_account(
            IdeScanner::import_github_copilot_from_local()?,
            &account,
            "GitHub Copilot",
        )?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_cursor_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_cursor_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 Cursor 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_windsurf_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_windsurf_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 Windsurf 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_kiro_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_kiro_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 Kiro 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_qoder_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_qoder_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 Qoder 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_trae_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_trae_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 Trae 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_codebuddy_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_codebuddy_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 CodeBuddy 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_codebuddy_cn_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_codebuddy_cn_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 CodeBuddy CN 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_workbuddy_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_workbuddy_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 WorkBuddy 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_zed_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        let scanned = IdeScanner::import_zed_from_local()?
            .into_iter()
            .next()
            .ok_or_else(|| "未读取到可用的 Zed 本地登录态".to_string())?;
        Self::merge_scanned_account(db, id, scanned)
    }

    pub fn refresh_all_by_platform(db: &Database, platform: &str) -> Result<usize, String> {
        let platform_lower = platform.to_ascii_lowercase();
        let target_ids = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|item| item.origin_platform.eq_ignore_ascii_case(&platform_lower))
            .map(|item| item.id)
            .collect::<Vec<_>>();

        let mut count = 0usize;
        for id in target_ids {
            let result = match platform_lower.as_str() {
                "vscode" => Self::refresh_vscode_account(db, &id),
                "github_copilot" => Self::refresh_github_copilot_account(db, &id),
                "cursor" => Self::refresh_cursor_account(db, &id),
                "windsurf" => Self::refresh_windsurf_account(db, &id),
                "kiro" => Self::refresh_kiro_account(db, &id),
                "qoder" => Self::refresh_qoder_account(db, &id),
                "trae" => Self::refresh_trae_account(db, &id),
                "codebuddy" => Self::refresh_codebuddy_account(db, &id),
                "codebuddy_cn" => Self::refresh_codebuddy_cn_account(db, &id),
                "workbuddy" => Self::refresh_workbuddy_account(db, &id),
                "zed" => Self::refresh_zed_account(db, &id),
                _ => Err(format!("{} 暂不支持本地刷新", platform)),
            };
            if result.is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn load_account(db: &Database, id: &str) -> Result<IdeAccount, String> {
        db.get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "IDE 账号不存在".to_string())
    }

    fn select_scanned_account(
        scanned_accounts: Vec<crate::services::ide_scanner::ScannedIdeAccount>,
        account: &IdeAccount,
        platform_label: &str,
    ) -> Result<crate::services::ide_scanner::ScannedIdeAccount, String> {
        if scanned_accounts.is_empty() {
            return Err(format!("未读取到可用的 {} 本地登录态", platform_label));
        }

        let expected_user_id = Self::extract_meta_string(account.meta_json.as_deref(), "user_id");
        let expected_login = Self::extract_meta_string(account.meta_json.as_deref(), "login");
        let expected_email = Self::normalize_optional_string(Some(account.email.as_str()));
        let expected_label = Self::normalize_optional_string(account.label.as_deref());
        let expected_access_token =
            Self::normalize_optional_string(Some(account.token.access_token.as_str()));
        let expected_refresh_token =
            Self::normalize_optional_string(Some(account.token.refresh_token.as_str()));

        for scanned in &scanned_accounts {
            if expected_user_id.as_ref().is_some_and(|expected| {
                Self::extract_meta_string(scanned.meta_json.as_deref(), "user_id").as_deref()
                    == Some(expected.as_str())
            }) {
                return Ok(scanned.clone());
            }
            if expected_login.as_ref().is_some_and(|expected| {
                Self::extract_meta_string(scanned.meta_json.as_deref(), "login").as_deref()
                    == Some(expected.as_str())
            }) {
                return Ok(scanned.clone());
            }
            if expected_access_token.as_ref().is_some_and(|expected| {
                Self::normalize_optional_string(scanned.access_token.as_deref()).as_deref()
                    == Some(expected.as_str())
            }) {
                return Ok(scanned.clone());
            }
            if expected_refresh_token.as_ref().is_some_and(|expected| {
                Self::normalize_optional_string(scanned.refresh_token.as_deref()).as_deref()
                    == Some(expected.as_str())
            }) {
                return Ok(scanned.clone());
            }
            if expected_email.as_ref().is_some_and(|expected| {
                Self::normalize_optional_string(Some(scanned.email.as_str())).as_deref()
                    == Some(expected.as_str())
            }) {
                return Ok(scanned.clone());
            }
            if expected_label.as_ref().is_some_and(|expected| {
                Self::normalize_optional_string(scanned.label.as_deref()).as_deref()
                    == Some(expected.as_str())
            }) {
                return Ok(scanned.clone());
            }
        }

        if scanned_accounts.len() == 1 {
            return scanned_accounts
                .into_iter()
                .next()
                .ok_or_else(|| format!("未读取到可用的 {} 本地登录态", platform_label));
        }

        Err(format!(
            "检测到多个 {} 本地登录态，但无法匹配当前账号，请重新导入对应账号后再试",
            platform_label
        ))
    }

    fn normalize_optional_string(raw: Option<&str>) -> Option<String> {
        raw.map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn extract_meta_string(raw: Option<&str>, key: &str) -> Option<String> {
        raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
            .and_then(|value| {
                value
                    .get(key)
                    .and_then(|item| item.as_str())
                    .map(str::to_string)
            })
            .and_then(|value| Self::normalize_optional_string(Some(value.as_str())))
    }

    fn merge_scanned_account(
        db: &Database,
        id: &str,
        scanned: crate::services::ide_scanner::ScannedIdeAccount,
    ) -> Result<IdeAccount, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "IDE 账号不存在".to_string())?;

        if !account
            .origin_platform
            .eq_ignore_ascii_case(&scanned.origin_platform)
        {
            return Err("账号平台与本地登录态不匹配".to_string());
        }

        if let Some(access_token) = scanned
            .access_token
            .filter(|value| !value.trim().is_empty())
        {
            account.token.access_token = access_token;
        }
        if let Some(refresh_token) = scanned
            .refresh_token
            .filter(|value| !value.trim().is_empty())
        {
            account.token.refresh_token = refresh_token;
        }
        if !scanned.email.trim().is_empty() {
            account.email = scanned.email;
        }
        if let Some(label) = scanned.label.filter(|value| !value.trim().is_empty()) {
            account.label = Some(label);
        }
        if let Some(meta_json) = scanned.meta_json.filter(|value| !value.trim().is_empty()) {
            account.meta_json = Some(meta_json);
        }

        let now = Utc::now();
        account.status = AccountStatus::Active;
        account.disabled_reason = None;
        account.token.updated_at = now;
        account.updated_at = now;
        account.last_used = now;

        db.upsert_ide_account(&account).map_err(|e| e.to_string())?;
        Ok(account)
    }
}
