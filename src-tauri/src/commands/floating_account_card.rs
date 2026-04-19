use crate::services::codex_instance_store::CodexInstanceStore;
use crate::services::event_bus::EventBus;
use crate::services::floating_account_card_store::{
    CreateFloatingAccountCardInput, FloatingAccountCard, FloatingAccountCardPatch,
    FloatingAccountCardStore,
};
use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))
}

fn list_valid_codex_instance_ids() -> Result<Vec<String>, String> {
    let mut ids = vec!["__default__".to_string()];
    for instance in CodexInstanceStore::list_instances()? {
        ids.push(instance.id);
    }
    Ok(ids)
}

fn emit_card_event(app: &AppHandle, event_name: &str, card: &FloatingAccountCard) {
    let _ = app.emit(event_name, card);
}

fn emit_deleted_event(app: &AppHandle, card_id: &str) {
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DeletedPayload {
        id: String,
        deleted_at: String,
    }
    let payload = DeletedPayload {
        id: card_id.to_string(),
        deleted_at: Utc::now().to_rfc3339(),
    };
    let _ = app.emit("floating.card.deleted", payload);
}

fn update_event_name(
    before: Option<&FloatingAccountCard>,
    after: &FloatingAccountCard,
) -> &'static str {
    if let Some(previous) = before {
        if previous.visible != after.visible {
            return "floating.card.visibility_changed";
        }
        if previous.x != after.x
            || previous.y != after.y
            || previous.width != after.width
            || previous.height != after.height
        {
            return "floating.card.position_changed";
        }
    }
    "floating.card.updated"
}

pub fn reconcile_floating_cards_instance_bindings(
    app: &AppHandle,
    source: &str,
) -> Result<Vec<FloatingAccountCard>, String> {
    let path = app_data_dir(app)?;
    let changed = FloatingAccountCardStore::reconcile_deleted_instances(
        &path,
        &list_valid_codex_instance_ids()?,
    )?;
    if !changed.is_empty() {
        for card in &changed {
            emit_card_event(app, "floating.card.updated", card);
        }
        EventBus::emit_data_changed(app, "floating_cards", "reconcile_instances", source);
    }
    Ok(changed)
}

pub fn emit_floating_account_changed(
    app: &AppHandle,
    platform: &str,
    account_id: Option<&str>,
    source: &str,
) {
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AccountChangedPayload {
        platform: String,
        account_id: Option<String>,
        source: String,
        changed_at: String,
    }
    let payload = AccountChangedPayload {
        platform: platform.trim().to_ascii_lowercase(),
        account_id: account_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        source: source.to_string(),
        changed_at: Utc::now().to_rfc3339(),
    };
    let _ = app.emit("floating.account.changed", payload);
}

#[tauri::command]
pub fn list_floating_account_cards(app: AppHandle) -> Result<Vec<FloatingAccountCard>, String> {
    reconcile_floating_cards_instance_bindings(&app, "floating.cards.list")?;
    FloatingAccountCardStore::list_cards(&app_data_dir(&app)?)
}

#[tauri::command]
pub fn create_floating_account_card(
    app: AppHandle,
    request: CreateFloatingAccountCardInput,
) -> Result<FloatingAccountCard, String> {
    let card = FloatingAccountCardStore::create_card(&app_data_dir(&app)?, request)?;
    emit_card_event(&app, "floating.card.created", &card);
    EventBus::emit_data_changed(&app, "floating_cards", "create", "floating.card.created");
    Ok(card)
}

#[tauri::command]
pub fn update_floating_account_card(
    app: AppHandle,
    id: String,
    patch: FloatingAccountCardPatch,
    expected_updated_at: Option<String>,
) -> Result<FloatingAccountCard, String> {
    let path = app_data_dir(&app)?;
    let before = FloatingAccountCardStore::list_cards(&path)?
        .into_iter()
        .find(|card| card.id == id);
    let updated =
        FloatingAccountCardStore::update_card(&path, &id, patch, expected_updated_at.as_deref())?;
    emit_card_event(&app, update_event_name(before.as_ref(), &updated), &updated);
    EventBus::emit_data_changed(&app, "floating_cards", "update", "floating.card.updated");
    Ok(updated)
}

#[tauri::command]
pub fn delete_floating_account_card(app: AppHandle, id: String) -> Result<bool, String> {
    let deleted = FloatingAccountCardStore::delete_card(&app_data_dir(&app)?, &id)?;
    if deleted {
        emit_deleted_event(&app, &id);
        EventBus::emit_data_changed(&app, "floating_cards", "delete", "floating.card.deleted");
    }
    Ok(deleted)
}
