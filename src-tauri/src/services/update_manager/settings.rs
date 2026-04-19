use super::types::CONFIG_FILE;
use super::{UpdateManager, UpdateReminderDecision, UpdateSettings};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

impl UpdateManager {
    pub fn load_settings(app_data_dir: &Path) -> Result<UpdateSettings, String> {
        let path = settings_path(app_data_dir);
        if !path.exists() {
            return Ok(UpdateSettings::default());
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("读取更新设置失败: {}", e))?;
        let mut settings =
            serde_json::from_str(&raw).map_err(|e| format!("解析更新设置失败: {}", e))?;
        let changed = normalize_settings(&mut settings);
        if changed {
            Self::save_settings(app_data_dir, &settings)?;
        }
        Ok(settings)
    }

    pub fn save_settings(app_data_dir: &Path, settings: &UpdateSettings) -> Result<(), String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let path = settings_path(app_data_dir);
        let mut normalized = settings.clone();
        normalize_settings(&mut normalized);
        let content = serde_json::to_string_pretty(&normalized)
            .map_err(|e| format!("序列化更新设置失败: {}", e))?;
        fs::write(path, content).map_err(|e| format!("写入更新设置失败: {}", e))
    }

    pub fn mark_checked_now(app_data_dir: &Path) -> Result<UpdateSettings, String> {
        let mut settings = Self::load_settings(app_data_dir)?;
        settings.last_check_at = Some(Utc::now().to_rfc3339());
        Self::save_settings(app_data_dir, &settings)?;
        Ok(settings)
    }

    pub fn mark_reminded_now(app_data_dir: &Path, version: &str) -> Result<UpdateSettings, String> {
        let mut settings = Self::load_settings(app_data_dir)?;
        settings.last_reminded_at = Some(Utc::now().to_rfc3339());
        settings.last_reminded_version = normalize_optional_string(Some(version));
        Self::save_settings(app_data_dir, &settings)?;
        Ok(settings)
    }

    pub fn evaluate_reminder_policy(
        app_data_dir: &Path,
        version: &str,
    ) -> Result<UpdateReminderDecision, String> {
        let settings = Self::load_settings(app_data_dir)?;
        Ok(evaluate_update_reminder_policy_with_now(
            &settings,
            version,
            Utc::now(),
        ))
    }
}

fn normalize_optional_string(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_silent_reminder_strategy(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "daily" => "daily".to_string(),
        "weekly" => "weekly".to_string(),
        _ => "immediate".to_string(),
    }
}

fn normalize_rfc3339(raw: Option<&str>) -> Option<String> {
    normalize_optional_string(raw).and_then(|value| {
        chrono::DateTime::parse_from_rfc3339(&value)
            .ok()
            .map(|parsed| parsed.with_timezone(&Utc).to_rfc3339())
    })
}

fn normalize_settings(settings: &mut UpdateSettings) -> bool {
    let mut changed = false;
    let normalized_skip_version = normalize_optional_string(settings.skip_version.as_deref());
    if settings.skip_version != normalized_skip_version {
        settings.skip_version = normalized_skip_version;
        changed = true;
    }
    let normalized_last_reminded_version =
        normalize_optional_string(settings.last_reminded_version.as_deref());
    if settings.last_reminded_version != normalized_last_reminded_version {
        settings.last_reminded_version = normalized_last_reminded_version;
        changed = true;
    }
    let normalized_strategy =
        normalize_silent_reminder_strategy(&settings.silent_reminder_strategy);
    if settings.silent_reminder_strategy != normalized_strategy {
        settings.silent_reminder_strategy = normalized_strategy;
        changed = true;
    }
    let normalized_last_check_at = normalize_rfc3339(settings.last_check_at.as_deref());
    if settings.last_check_at != normalized_last_check_at {
        settings.last_check_at = normalized_last_check_at;
        changed = true;
    }
    let normalized_last_reminded_at = normalize_rfc3339(settings.last_reminded_at.as_deref());
    if settings.last_reminded_at != normalized_last_reminded_at {
        settings.last_reminded_at = normalized_last_reminded_at;
        changed = true;
    }
    changed
}

pub(super) fn evaluate_update_reminder_policy_with_now(
    settings: &UpdateSettings,
    version: &str,
    now: chrono::DateTime<Utc>,
) -> UpdateReminderDecision {
    let mut normalized = settings.clone();
    normalize_settings(&mut normalized);
    let version = normalize_optional_string(Some(version));
    let Some(version) = version else {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "invalid_version".to_string(),
            settings: normalized,
        };
    };

    if normalized
        .skip_version
        .as_ref()
        .is_some_and(|skip| skip.eq_ignore_ascii_case(&version))
    {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "skipped_version".to_string(),
            settings: normalized,
        };
    }

    if normalized.disable_reminders {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "reminders_disabled".to_string(),
            settings: normalized,
        };
    }

    if normalized.silent_reminder_strategy == "immediate" {
        return UpdateReminderDecision {
            should_notify: true,
            reason: "allow_immediate".to_string(),
            settings: normalized,
        };
    }

    if normalized
        .last_reminded_version
        .as_ref()
        .is_none_or(|last| !last.eq_ignore_ascii_case(&version))
    {
        return UpdateReminderDecision {
            should_notify: true,
            reason: "allow_new_version".to_string(),
            settings: normalized,
        };
    }

    let Some(last_reminded_at) = normalized
        .last_reminded_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|dt| dt.with_timezone(&Utc))
    else {
        return UpdateReminderDecision {
            should_notify: true,
            reason: "allow_missing_history".to_string(),
            settings: normalized,
        };
    };

    let required_interval_secs = if normalized.silent_reminder_strategy == "weekly" {
        7 * 24 * 60 * 60
    } else {
        24 * 60 * 60
    };
    if (now - last_reminded_at).num_seconds() < required_interval_secs {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "silent_window_active".to_string(),
            settings: normalized,
        };
    }

    UpdateReminderDecision {
        should_notify: true,
        reason: "allow_window_expired".to_string(),
        settings: normalized,
    }
}

pub(super) fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONFIG_FILE)
}
