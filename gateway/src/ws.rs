//! WebSocket endpoint: streams the engine's coalesced book snapshots, trades, and order events.
//!
//! On connect, the client receives the current book snapshot immediately, then live updates as
//! they occur. Book updates are already throttled to ~60fps on the engine thread, so no extra
//! coalescing is needed here.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tokio::sync::broadcast::error::RecvError;

use crate::dto::ServerMessage;
use crate::state::AppState;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_loop(socket, state))
}

async fn client_loop(mut socket: WebSocket, state: AppState) {
    let mut rx = state.broadcast.subscribe();

    // Send the current book immediately so a fresh client isn't blank until the next change.
    if let Some(book) = state.latest_book.read().ok().and_then(|g| g.clone()) {
        if !send(&mut socket, &book).await {
            return;
        }
    }

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if !send(&mut socket, &msg).await {
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => {
                // Slow client fell behind; resync with the latest book and continue.
                if let Some(book) = state.latest_book.read().ok().and_then(|g| g.clone()) {
                    if !send(&mut socket, &book).await {
                        break;
                    }
                }
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// Serialize and send one message; returns `false` if the socket is gone.
async fn send(socket: &mut WebSocket, msg: &ServerMessage) -> bool {
    let Ok(json) = serde_json::to_string(msg) else {
        return true; // skip an unserializable message rather than drop the connection
    };
    socket.send(Message::Text(json)).await.is_ok()
}
