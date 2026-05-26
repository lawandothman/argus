//! `WS /api/stream` — pushes live telemetry events to subscribers.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;

pub(super) async fn stream(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| pump(socket, state))
}

async fn pump(mut socket: WebSocket, state: AppState) {
    let mut events = state.events.subscribe();
    loop {
        match events.recv().await {
            Ok(event) => {
                let Ok(json) = serde_json::to_string(&event) else {
                    continue;
                };
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break; // client went away
                }
            }
            Err(RecvError::Lagged(_)) => continue, // slow client: skip ahead
            Err(RecvError::Closed) => break,
        }
    }
}
