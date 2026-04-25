use serde::Serialize;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

const INTERNAL_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataChangedPayload {
    pub domain: String,
    pub action: String,
    pub source: String,
}

static INTERNAL_CHANNEL: OnceLock<broadcast::Sender<DataChangedPayload>> = OnceLock::new();

fn internal_channel() -> &'static broadcast::Sender<DataChangedPayload> {
    INTERNAL_CHANNEL.get_or_init(|| {
        let (tx, _) = broadcast::channel(INTERNAL_CHANNEL_CAPACITY);
        tx
    })
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
        crate::services::websocket::broadcast_data_changed(payload.clone());
        // 后端服务订阅入口（Wakeup listener / 后续可扩展）
        let _ = internal_channel().send(payload);
    }

    /// 后端订阅端：拿到一个 broadcast::Receiver，可在守护进程里 await recv()。
    pub fn subscribe() -> broadcast::Receiver<DataChangedPayload> {
        internal_channel().subscribe()
    }
}
