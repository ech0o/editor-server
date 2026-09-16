use crate::kafka::JobEvent;
use crate::state::AppState;
use crate::websocket::manager::WsManager;
use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

pub async fn job_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Response {
    let ws_manager = state.ws_manager.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, ws_manager, job_id))
}

async fn handle_socket(mut socket: WebSocket, manager: Arc<WsManager>, job_id: Uuid) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<JobEvent>(16);
    manager.add(job_id, tx).await;
    while let Some(event) = rx.recv().await {
        let text = serde_json::to_string(&event).unwrap();

        if socket.send(Message::Text(text.into())).await.is_err() {
            break;
        }
        if event.status.is_finished() {
            break;
        }
    }
    manager.remove(job_id).await;
}
