use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataChangedPayload {
    pub domain: String,
    pub action: String,
    pub source: String,
}

pub struct EventBus;

impl EventBus {
    pub fn emit_data_changed(app: &AppHandle, domain: &str, action: &str, source: &str) {
        let payload = DataChangedPayload {
            domain: domain.to_string(),
            action: action.to_string(),
            source: source.to_string(),
        };
        let _ = app.emit("data:changed", &payload);
        crate::services::websocket::broadcast_data_changed(payload);
    }
}
