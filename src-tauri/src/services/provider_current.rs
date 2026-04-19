use crate::db::Database;

mod helpers;
mod platforms;

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
                label: matched
                    .and_then(|item| item.label.clone())
                    .or_else(|| matched.map(|item| item.email.clone())),
                email: matched.map(|item| item.email.clone()),
                status: matched.map(|item| format!("{:?}", item.status).to_lowercase()),
            });
        }
        Ok(snapshots)
    }
}
