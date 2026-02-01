use std::path::Path;

use anyhow::Context as _;
use axum::{
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use tokio::io::AsyncBufReadExt as _;
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

    let http_app = axum::Router::new().route("/ws", axum::routing::any(ws_endpoint_handler));

    let http_listener = tokio::net::TcpListener::bind("0.0.0.0:2552").await?;
    let http_server_fut = async {
        axum::serve(http_listener, http_app)
            .await
            .map_err(anyhow::Error::from)
    };

    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").context("$XDG_RUNTIME_DIR not set")?;
    let unix_socket = tokio::net::UnixListener::bind(Path::new(&runtime_dir).join("weird.sock"))
        .context("failed to bind weird.sock")?;
    let unix_socket_fut = serve_unix_socket(unix_socket);

    tokio::try_join!(http_server_fut, unix_socket_fut)?;

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

async fn serve_unix_socket(socket: tokio::net::UnixListener) -> anyhow::Result<()> {
    loop {
        let (conn, _addr) = socket.accept().await?;
        tokio::spawn(handle_unix_conn(conn));
    }
}

async fn handle_unix_conn(mut conn: tokio::net::UnixStream) -> anyhow::Result<()> {
    tracing::info!("client connected");

    let (rx, _tx) = conn.split();

    let rx = tokio::io::BufReader::new(rx);
    let mut rx_lines = rx.lines();
    loop {
        let line = rx_lines.next_line().await;
        let line = match line {
            Ok(Some(line)) => line,
            Ok(None) => {
                tracing::info!("client disconnected");
                break;
            }
            Err(error) => {
                tracing::warn!("unix client error: {error}");
                break;
            }
        };

        tracing::info!("received unix client message: {line}");
    }

    Ok(())
}
