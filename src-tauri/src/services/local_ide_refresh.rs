use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use crate::services::ide_scanner::IdeScanner;
use chrono::Utc;

pub struct LocalIdeRefreshService;

impl LocalIdeRefreshService {
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
