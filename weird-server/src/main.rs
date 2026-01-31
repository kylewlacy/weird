use axum::{
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_TRACING_DIRECTIVE: &str = concat!(env!("CARGO_CRATE_NAME"), "=info,warn");
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_TRACING_DIRECTIVE)),
        )
        .init();

    let app = axum::Router::new().route("/ws", axum::routing::any(ws_endpoint_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2552").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ws_endpoint_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws_handler)
}

async fn ws_handler(mut socket: WebSocket) {
    tracing::info!("client connected");

    while let Some(message) = socket.recv().await {
        let message = match message {
            Err(error) => {
                tracing::warn!("received client error: {error}");
                break;
            }
            Ok(Message::Close(close_frame)) => {
                if let Some(close) = close_frame {
                    tracing::info!(
                        code = close.code,
                        reason = close.reason.as_str(),
                        "client closed connection"
                    );
                } else {
                    tracing::info!("client closed connection (no close frame)");
                }
                break;
            }
            Ok(Message::Ping(_) | Message::Pong(_)) => {
                continue;
            }
            Ok(message @ (Message::Text(_) | Message::Binary(_))) => message,
        };

        let result = socket.send(message).await;
        match result {
            Ok(()) => {}
            Err(error) => {
                tracing::warn!("failed to send client message: {error}");
                break;
            }
        }
    }
}
