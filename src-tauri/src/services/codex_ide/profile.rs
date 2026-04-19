use super::helpers::{
    extract_chatgpt_organization_id_from_access_token, get_meta_string, normalize_string,
    resolve_codex_account_id,
};
use super::{CodexProfile, WindowInfo};
use crate::models::IdeAccount;
use chrono::Utc;
use serde_json::{Map, Value};

pub(super) fn normalize_remaining_percentage(window: &WindowInfo) -> i32 {
    100 - window.used_percent.unwrap_or(0).clamp(0, 100)
}

pub(super) fn normalize_window_minutes(window: &WindowInfo) -> Option<i64> {
    let seconds = window.limit_window_seconds?;
    if seconds <= 0 {
        return None;
    }
    Some((seconds + 59) / 60)
}

pub(super) fn normalize_reset_time(window: &WindowInfo) -> Option<i64> {
    if let Some(reset_at) = window.reset_at {
        return Some(reset_at);
    }
    let reset_after_seconds = window.reset_after_seconds?;
    if reset_after_seconds < 0 {
        return None;
    }
    Some(Utc::now().timestamp() + reset_after_seconds)
}

pub(super) fn parse_remote_profile(
    payload: &Value,
    account: &IdeAccount,
    meta: &Map<String, Value>,
) -> CodexProfile {
    let records = collect_account_records(payload);
    if records.is_empty() {
        return CodexProfile {
            account_name: None,
            account_structure: None,
            account_id: None,
        };
    }

    let ordering_first_id = payload
        .get("account_ordering")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let expected_account_id = resolve_codex_account_id(account, meta);
    let expected_org_id =
        normalize_string(get_meta_string(meta, "organization_id").or_else(|| {
            extract_chatgpt_organization_id_from_access_token(&account.token.access_token)
        }));

    let selected = records
        .iter()
        .find(|record| {
            expected_account_id.as_ref().is_some_and(|expected| {
                extract_account_record_field(
                    record,
                    &["id", "account_id", "chatgpt_account_id", "workspace_id"],
                )
                .is_some_and(|candidate| candidate == *expected)
            })
        })
        .cloned()
        .or_else(|| {
            records
                .iter()
                .find(|record| {
                    ordering_first_id.as_ref().is_some_and(|expected| {
                        extract_account_record_field(
                            record,
                            &["id", "account_id", "chatgpt_account_id", "workspace_id"],
                        )
                        .is_some_and(|candidate| candidate == *expected)
                    })
                })
                .cloned()
        })
        .or_else(|| {
            records
                .iter()
                .find(|record| {
                    expected_org_id.as_ref().is_some_and(|expected| {
                        extract_account_record_field(
                            record,
                            &["organization_id", "org_id", "workspace_id"],
                        )
                        .is_some_and(|candidate| candidate == *expected)
                    })
                })
                .cloned()
        })
        .unwrap_or_else(|| records[0].clone());

    CodexProfile {
        account_name: extract_account_record_field(
            &selected,
            &[
                "name",
                "display_name",
                "account_name",
                "organization_name",
                "workspace_name",
                "title",
            ],
        ),
        account_structure: extract_account_record_field(
            &selected,
            &[
                "structure",
                "account_structure",
                "kind",
                "type",
                "account_type",
            ],
        ),
        account_id: extract_account_record_field(
            &selected,
            &["id", "account_id", "chatgpt_account_id", "workspace_id"],
        ),
    }
}

fn collect_account_records(value: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    collect_account_records_inner(value, &mut out);
    out
}

fn collect_account_records_inner(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_account_records_inner(item, out);
            }
        }
        Value::Object(map) => {
            if looks_like_account_record(map) {
                out.push(value.clone());
            }
            for nested in map.values() {
                collect_account_records_inner(nested, out);
            }
        }
        _ => {}
    }
}

fn looks_like_account_record(map: &Map<String, Value>) -> bool {
    let id_like = ["id", "account_id", "chatgpt_account_id", "workspace_id"]
        .iter()
        .any(|key| map.get(*key).and_then(|value| value.as_str()).is_some());
    let name_like = [
        "name",
        "display_name",
        "account_name",
        "organization_name",
        "workspace_name",
        "title",
    ]
    .iter()
    .any(|key| map.get(*key).and_then(|value| value.as_str()).is_some());
    id_like || name_like
}

fn extract_account_record_field(record: &Value, keys: &[&str]) -> Option<String> {
    let object = record.as_object()?;
    for key in keys {
        if let Some(value) = object.get(*key).and_then(|value| value.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}
