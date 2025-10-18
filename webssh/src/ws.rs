use std::time::Duration;

use axum::{extract::ws::{WebSocketUpgrade, Message, WebSocket}, response::IntoResponse};
use futures_util::{stream::StreamExt, SinkExt};
use tokio::time::sleep;
use tracing::{info, warn};

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("ws connected");

    if let Err(e) = socket.send(Message::Text("connected (echo server stub)".into())).await {
        warn!(?e, "failed to send greeting");
        return;
    }

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(t) => {
                if t == "ping" {
                    let _ = socket.send(Message::Text("pong".into())).await;
                } else {
                    let echo = format!("echo: {}", t);
                    let _ = socket.send(Message::Text(echo)).await;
                }
            }
            Message::Binary(b) => {
                let mut r = b;
                r.reverse();
                let _ = socket.send(Message::Binary(r)).await;
            }
            Message::Ping(v) => {
                let _ = socket.send(Message::Pong(v)).await;
            }
            Message::Close(frame) => {
                info!(?frame, "ws closed by client");
                break;
            }
            _ => {}
        }
        // simple backoff to avoid tight loops in this stub
        sleep(Duration::from_millis(2)).await;
    }
    info!("ws disconnected");
}
