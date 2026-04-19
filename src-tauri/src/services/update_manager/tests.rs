use super::*;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

fn unique_temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ais-update-manager-{}-{}",
        label,
        uuid::Uuid::new_v4()
    ));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn load_settings_migrates_legacy_fields() {
    let dir = unique_temp_dir("migrate");
    let legacy = r#"{
  "auto_check": true,
  "auto_install": false,
  "last_check_at": "2026-04-16T01:00:00Z"
}"#;
    fs::write(settings::settings_path(&dir), legacy).expect("write legacy settings");
    let loaded = UpdateManager::load_settings(&dir).expect("load settings");
    assert!(loaded.auto_check);
    assert!(!loaded.auto_install);
    assert_eq!(loaded.skip_version, None);
    assert_eq!(loaded.disable_reminders, false);
    assert_eq!(loaded.silent_reminder_strategy, "immediate");
    assert_eq!(loaded.last_reminded_at, None);
    assert_eq!(loaded.last_reminded_version, None);
}

#[test]
fn reminder_policy_respects_skipped_version() {
    let settings = UpdateSettings {
        skip_version: Some("0.1.12".to_string()),
        ..UpdateSettings::default()
    };
    let decision = settings::evaluate_update_reminder_policy_with_now(
        &settings,
        "0.1.12",
        Utc::now(),
    );
    assert!(!decision.should_notify);
    assert_eq!(decision.reason, "skipped_version");
}

#[test]
fn reminder_policy_blocks_daily_window_for_same_version() {
    let now = Utc::now();
    let settings = UpdateSettings {
        silent_reminder_strategy: "daily".to_string(),
        last_reminded_version: Some("0.1.13".to_string()),
        last_reminded_at: Some((now - chrono::Duration::hours(6)).to_rfc3339()),
        ..UpdateSettings::default()
    };
    let decision = settings::evaluate_update_reminder_policy_with_now(
        &settings,
        "0.1.13",
        now,
    );
    assert!(!decision.should_notify);
    assert_eq!(decision.reason, "silent_window_active");
}

#[test]
fn reminder_policy_allows_newer_version_even_in_silent_window() {
    let now = Utc::now();
    let settings = UpdateSettings {
        silent_reminder_strategy: "weekly".to_string(),
        last_reminded_version: Some("0.1.13".to_string()),
        last_reminded_at: Some((now - chrono::Duration::hours(6)).to_rfc3339()),
        ..UpdateSettings::default()
    };
    let decision = settings::evaluate_update_reminder_policy_with_now(
        &settings,
        "0.1.14",
        now,
    );
    assert!(decision.should_notify);
    assert_eq!(decision.reason, "allow_new_version");
}
