use super::Database;
use chrono::{DateTime, Utc};
use rusqlite::Result as SqlResult;
use std::str::FromStr;

fn parse_account_status(status_str: &str) -> crate::models::AccountStatus {
    match status_str {
        "forbidden" => crate::models::AccountStatus::Forbidden,
        "rate_limited" => crate::models::AccountStatus::RateLimited,
        "expired" => crate::models::AccountStatus::Expired,
        "unknown" => crate::models::AccountStatus::Unknown,
        _ => crate::models::AccountStatus::Active,
    }
}

fn serialize_account_status(status: crate::models::AccountStatus) -> &'static str {
    match status {
        crate::models::AccountStatus::Active => "active",
        crate::models::AccountStatus::Expired => "expired",
        crate::models::AccountStatus::Forbidden => "forbidden",
        crate::models::AccountStatus::RateLimited => "rate_limited",
        crate::models::AccountStatus::Unknown => "unknown",
    }
}

fn map_ide_account_row(row: &rusqlite::Row<'_>) -> SqlResult<crate::models::IdeAccount> {
    let machine_id: Option<String> = row.get(10)?;
    let profile = if let Some(mid) = machine_id {
        Some(crate::models::DeviceProfile {
            machine_id: mid,
            mac_machine_id: row.get(11)?,
            dev_device_id: row.get(12)?,
            sqm_id: row.get(13)?,
        })
    } else {
        None
    };

    let status_str: String = row.get(7)?;
    let tags_json: Option<String> = row.get(20).ok();
    let tags: Vec<String> = tags_json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();

    let disabled_at_str: Option<String> = row.get(22).ok().flatten();
    let disabled_at = disabled_at_str
        .as_deref()
        .and_then(|s| DateTime::<Utc>::from_str(s).ok());
    let fingerprint_id: Option<String> = row.get(23).ok().flatten();
    let quota_error_json: Option<String> = row.get(24).ok().flatten();

    Ok(crate::models::IdeAccount {
        id: row.get(0)?,
        email: row.get(1)?,
        origin_platform: row.get(2)?,
        token: crate::models::OAuthToken {
            access_token: row.get(3)?,
            refresh_token: row.get(4)?,
            expires_in: row.get(5)?,
            token_type: row.get(6)?,
            updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(18)?)
                .unwrap_or_else(|_| Utc::now()),
        },
        status: parse_account_status(&status_str),
        disabled_reason: row.get(8)?,
        is_proxy_disabled: row.get(9)?,
        created_at: DateTime::<Utc>::from_str(&row.get::<_, String>(17)?)
            .unwrap_or_else(|_| Utc::now()),
        updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(18)?)
            .unwrap_or_else(|_| Utc::now()),
        last_used: DateTime::<Utc>::from_str(&row.get::<_, String>(19)?)
            .unwrap_or_else(|_| Utc::now()),
        device_profile: profile,
        quota_json: row.get(14)?,
        project_id: row.get(15)?,
        meta_json: row.get(16)?,
        label: row.get(21).ok(),
        tags,
        disabled_at,
        fingerprint_id,
        quota_error_json,
    })
}

impl Database {
    #[allow(dead_code)]
    pub fn get_active_ide_accounts(
        &self,
        origin_platform: &str,
    ) -> SqlResult<Vec<crate::models::IdeAccount>> {
        self.query_rows(
            "SELECT
                id, email, origin_platform, access_token, refresh_token, expires_in, token_type,
                status, disabled_reason, is_proxy_disabled, machine_id, mac_machine_id,
                dev_device_id, sqm_id, quota_json, project_id, meta_json, created_at, updated_at, last_used, tags, label,
                disabled_at, fingerprint_id, quota_error_json
             FROM ide_accounts
             WHERE origin_platform = ? AND status = 'active' AND is_proxy_disabled = 0",
            &[&origin_platform],
            map_ide_account_row,
        )
    }

    #[allow(dead_code)]
    pub fn update_ide_account_status(
        &self,
        id: &str,
        status: crate::models::AccountStatus,
        reason: Option<&str>,
    ) -> SqlResult<usize> {
        let status_str = serialize_account_status(status);
        self.execute(
            "UPDATE ide_accounts SET status = ?, disabled_reason = ?, updated_at = datetime('now') WHERE id = ?",
            &[&status_str, &reason, &id],
        )
    }

    pub fn get_all_ide_accounts(&self) -> SqlResult<Vec<crate::models::IdeAccount>> {
        self.query_rows(
            "SELECT
                id, email, origin_platform, access_token, refresh_token, expires_in, token_type,
                status, disabled_reason, is_proxy_disabled, machine_id, mac_machine_id,
                dev_device_id, sqm_id, quota_json, project_id, meta_json, created_at, updated_at, last_used, tags, label,
                disabled_at, fingerprint_id, quota_error_json
             FROM ide_accounts ORDER BY created_at DESC",
            &[],
            map_ide_account_row,
        )
    }

    pub fn upsert_ide_account(&self, acc: &crate::models::IdeAccount) -> SqlResult<usize> {
        let status_str = serialize_account_status(acc.status.clone());
        let (mid, mac, did, sqm) = match &acc.device_profile {
            Some(p) => (
                Some(&p.machine_id),
                Some(&p.mac_machine_id),
                Some(&p.dev_device_id),
                Some(&p.sqm_id),
            ),
            None => (None, None, None, None),
        };

        let c_at = acc.created_at.to_rfc3339();
        let u_at = acc.updated_at.to_rfc3339();
        let lu = acc.last_used.to_rfc3339();
        let disabled_at = acc.disabled_at.map(|dt| dt.to_rfc3339());

        self.execute(
            "INSERT INTO ide_accounts (
                id, email, origin_platform, access_token, refresh_token, expires_in, token_type,
                status, disabled_reason, is_proxy_disabled, machine_id, mac_machine_id,
                dev_device_id, sqm_id, quota_json, project_id, meta_json, created_at, updated_at, last_used, label,
                disabled_at, fingerprint_id, quota_error_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
            ON CONFLICT(id) DO UPDATE SET
                email = excluded.email,
                origin_platform = excluded.origin_platform,
                access_token = excluded.access_token,
                refresh_token = excluded.refresh_token,
                expires_in = excluded.expires_in,
                token_type = excluded.token_type,
                status = excluded.status,
                disabled_reason = excluded.disabled_reason,
                is_proxy_disabled = excluded.is_proxy_disabled,
                machine_id = excluded.machine_id,
                mac_machine_id = excluded.mac_machine_id,
                dev_device_id = excluded.dev_device_id,
                sqm_id = excluded.sqm_id,
                quota_json = excluded.quota_json,
                project_id = excluded.project_id,
                meta_json = excluded.meta_json,
                label = excluded.label,
                updated_at = excluded.updated_at,
                last_used = excluded.last_used,
                disabled_at = excluded.disabled_at,
                fingerprint_id = excluded.fingerprint_id,
                quota_error_json = excluded.quota_error_json",
            rusqlite::params![
                acc.id, acc.email, acc.origin_platform, acc.token.access_token, acc.token.refresh_token,
                acc.token.expires_in, acc.token.token_type, status_str, acc.disabled_reason,
                acc.is_proxy_disabled, mid, mac, did, sqm, acc.quota_json, acc.project_id, acc.meta_json,
                c_at, u_at, lu, acc.label, disabled_at, acc.fingerprint_id, acc.quota_error_json
            ],
        )
    }

    pub fn delete_ide_account(&self, id: &str) -> SqlResult<usize> {
        self.execute("DELETE FROM ide_accounts WHERE id = ?", &[&id])
    }

    pub fn update_ide_account_tags(&self, id: &str, tags_json: &str) -> SqlResult<usize> {
        self.execute(
            "UPDATE ide_accounts SET tags = ?, updated_at = datetime('now') WHERE id = ?",
            &[&tags_json, &id],
        )
    }

    pub fn update_ide_account_project_id(
        &self,
        id: &str,
        project_id: Option<&str>,
    ) -> SqlResult<usize> {
        self.execute(
            "UPDATE ide_accounts SET project_id = ?, updated_at = datetime('now') WHERE id = ?",
            &[&project_id, &id],
        )
    }

    pub fn update_ide_account_label(&self, id: &str, label: Option<&str>) -> SqlResult<usize> {
        self.execute(
            "UPDATE ide_accounts SET label = ?, updated_at = datetime('now') WHERE id = ?",
            &[&label, &id],
        )
    }

    pub fn mark_ide_account_disabled(
        &self,
        id: &str,
        reason: &str,
    ) -> SqlResult<usize> {
        let now = Utc::now().to_rfc3339();
        self.execute(
            "UPDATE ide_accounts SET status = 'forbidden', disabled_reason = ?, disabled_at = ?, updated_at = ? WHERE id = ?",
            &[&reason, &now, &now, &id],
        )
    }

    pub fn clear_ide_account_disabled(&self, id: &str) -> SqlResult<usize> {
        let now = Utc::now().to_rfc3339();
        self.execute(
            "UPDATE ide_accounts SET status = 'active', disabled_reason = NULL, disabled_at = NULL, updated_at = ? WHERE id = ?",
            &[&now, &id],
        )
    }

    pub fn update_ide_account_fingerprint(
        &self,
        id: &str,
        fingerprint_id: Option<&str>,
    ) -> SqlResult<usize> {
        let now = Utc::now().to_rfc3339();
        self.execute(
            "UPDATE ide_accounts SET fingerprint_id = ?, updated_at = ? WHERE id = ?",
            &[&fingerprint_id, &now, &id],
        )
    }

    pub fn update_ide_account_quota_error(
        &self,
        id: &str,
        quota_error_json: Option<&str>,
    ) -> SqlResult<usize> {
        let now = Utc::now().to_rfc3339();
        self.execute(
            "UPDATE ide_accounts SET quota_error_json = ?, updated_at = ? WHERE id = ?",
            &[&quota_error_json, &now, &id],
        )
    }
}
