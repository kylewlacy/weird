use std::sync::Arc;

use axum::{
    extract::{WebSocketUpgrade, ws},
    response::IntoResponse,
};
use tokio::sync::RwLock;

use crate::world::World;

#[derive(Clone)]
pub struct AppState {
    #[expect(unused)]
    pub world: Arc<RwLock<World>>,
}

pub fn router(state: AppState) -> axum::Router {
    let router = axum::Router::new()
        .route("/ws", axum::routing::any(ws_endpoint_handler))
        .with_state(state);

    router
}

async fn ws_endpoint_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws_handler)
}

async fn ws_handler(mut socket: ws::WebSocket) {
    tracing::info!("web client connected");

    while let Some(message) = socket.recv().await {
        let message = match message {
            Err(error) => {
                tracing::warn!("received web client error: {error}");
                break;
            }
            Ok(ws::Message::Close(close_frame)) => {
                if let Some(close) = close_frame {
                    tracing::info!(
                        code = close.code,
                        reason = close.reason.as_str(),
                        "web client closed connection"
                    );
                } else {
                    tracing::info!("web client closed connection (no close frame)");
                }
                break;
            }
            Ok(ws::Message::Ping(_) | ws::Message::Pong(_)) => {
                continue;
            }
            Ok(message @ (ws::Message::Text(_) | ws::Message::Binary(_))) => message,
        };

        let result = socket.send(message).await;
        match result {
            Ok(()) => {}
            Err(error) => {
                tracing::warn!("failed to send web client message: {error}");
                break;
            }
        }
    }
}
