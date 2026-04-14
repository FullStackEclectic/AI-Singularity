use crate::services::event_bus::DataChangedPayload;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::{Arc, OnceLock, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};

const PORT_RANGE_START: u16 = 32145;
const PORT_RANGE_LEN: u16 = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
enum WsMessage {
    #[serde(rename = "event.ready")]
    Ready { version: String },
    #[serde(rename = "event.data_changed")]
    DataChanged(DataChangedPayload),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub client_count: usize,
}

struct WsServer {
    tx: broadcast::Sender<String>,
    port: RwLock<Option<u16>>,
    client_count: RwLock<usize>,
}

impl WsServer {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            tx,
            port: RwLock::new(None),
            client_count: RwLock::new(0),
        }
    }
}

static WS_SERVER: OnceLock<Arc<WsServer>> = OnceLock::new();

fn server() -> &'static Arc<WsServer> {
    WS_SERVER.get_or_init(|| Arc::new(WsServer::new()))
}

pub async fn start_server() {
    let srv = server().clone();
    if srv.port.read().ok().and_then(|guard| *guard).is_some() {
        return;
    }

    let mut listener = None;
    let mut bound_port = None;
    for offset in 0..PORT_RANGE_LEN {
        let port = PORT_RANGE_START + offset;
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(value) => {
                listener = Some(value);
                bound_port = Some(port);
                break;
            }
            Err(_) => continue,
        }
    }

    let Some(listener) = listener else {
        tracing::warn!("[WebSocket] 无法绑定本地端口 {}-{}", PORT_RANGE_START, PORT_RANGE_START + PORT_RANGE_LEN - 1);
        return;
    };
    if let Ok(mut guard) = srv.port.write() {
        *guard = bound_port;
    }
    tracing::info!("[WebSocket] 本地广播服务已启动: ws://127.0.0.1:{}", bound_port.unwrap_or(PORT_RANGE_START));

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let cloned = srv.clone();
                tokio::spawn(async move {
                    let _ = handle_connection(cloned, stream).await;
                });
            }
            Err(err) => {
                tracing::warn!("[WebSocket] 接受连接失败: {}", err);
                break;
            }
        }
    }
}

async fn handle_connection(server: Arc<WsServer>, stream: TcpStream) -> Result<(), String> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| format!("握手失败: {}", e))?;
    let (mut write, mut read) = ws_stream.split();
    let mut rx = server.tx.subscribe();

    if let Ok(mut guard) = server.client_count.write() {
        *guard += 1;
    }

    let ready = serde_json::to_string(&WsMessage::Ready {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
    .map_err(|e| format!("序列化 ready 事件失败: {}", e))?;
    write
        .send(Message::Text(ready))
        .await
        .map_err(|e| format!("发送 ready 事件失败: {}", e))?;

    loop {
        tokio::select! {
            outgoing = rx.recv() => {
                match outgoing {
                    Ok(text) => {
                        if write.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            incoming = read.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = write.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }

    if let Ok(mut guard) = server.client_count.write() {
        *guard = guard.saturating_sub(1);
    }

    Ok(())
}

pub fn broadcast_data_changed(payload: DataChangedPayload) {
    let message = match serde_json::to_string(&WsMessage::DataChanged(payload)) {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!("[WebSocket] 序列化 data_changed 事件失败: {}", err);
            return;
        }
    };
    let _ = server().tx.send(message);
}

pub fn get_status() -> WebSocketStatus {
    let srv = server();
    WebSocketStatus {
        running: srv.port.read().ok().and_then(|guard| *guard).is_some(),
        port: srv.port.read().ok().and_then(|guard| *guard),
        client_count: srv.client_count.read().map(|guard| *guard).unwrap_or(0),
    }
}
