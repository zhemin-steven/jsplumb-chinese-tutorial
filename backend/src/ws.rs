use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    response::IntoResponse,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures::{SinkExt, StreamExt};
use tracing::{debug, info};

use crate::AppState;

pub async fn ws_handler(
    State(_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!(client = %addr, "ws connect");
    ws.on_upgrade(|socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, addr: SocketAddr) {
    debug!(client = %addr, "ws upgraded (stub)");
    // Simple stub: send a welcome message and close.
    let _ = socket.send(Message::Text("welcome".into())).await;
    let _ = socket.close().await;
    // Drain any remaining messages gracefully
    let mut _stream = socket.split().1;
    while let Some(_msg) = _stream.next().await {
        // ignore
    }
}
