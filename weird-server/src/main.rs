use std::path::Path;

use anyhow::Context as _;
use axum::{
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt};
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
    let unix_socket_path = Path::new(&runtime_dir).join("weird.sock");
    try_clean_up_old_socket(&unix_socket_path).await?;
    let unix_socket =
        tokio::net::UnixListener::bind(unix_socket_path).context("failed to bind weird.sock")?;
    let unix_socket_fut = serve_unix_socket(unix_socket);

    tokio::try_join!(http_server_fut, unix_socket_fut)?;

    Ok(())
}

async fn ws_endpoint_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws_handler)
}

async fn ws_handler(mut socket: WebSocket) {
    tracing::info!("web client connected");

    while let Some(message) = socket.recv().await {
        let message = match message {
            Err(error) => {
                tracing::warn!("received web client error: {error}");
                break;
            }
            Ok(Message::Close(close_frame)) => {
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
            Ok(Message::Ping(_) | Message::Pong(_)) => {
                continue;
            }
            Ok(message @ (Message::Text(_) | Message::Binary(_))) => message,
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

async fn serve_unix_socket(socket: tokio::net::UnixListener) -> anyhow::Result<()> {
    loop {
        let (conn, _addr) = socket.accept().await?;
        tokio::spawn(handle_unix_conn(conn));
    }
}

async fn handle_unix_conn(mut conn: tokio::net::UnixStream) -> anyhow::Result<()> {
    tracing::info!("unix client connected");

    let (rx, _tx) = conn.split();

    let rx = tokio::io::BufReader::new(rx);
    let mut rx_lines = rx.lines();
    loop {
        let line = rx_lines.next_line().await;
        let line = match line {
            Ok(Some(line)) => line,
            Ok(None) => {
                tracing::info!("unix client disconnected");
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

/// Try to clean up a Unix socket that we want to bind to.
///
/// We'll check if the socket path already exists by connecting to it. If it
/// does exist and we can connect to it successfully, that means another
/// instance of the server is probably still running. If the socket exists
/// but we can't connect to it, that probably means a previous instance of
/// the server exited, so we try to remove the socket so we can bind a new one.
async fn try_clean_up_old_socket(path: &Path) -> anyhow::Result<()> {
    let result = tokio::net::UnixStream::connect(path).await;

    match result {
        Ok(mut conn) => {
            // Connected to socket, this means there's probably a server
            // already listening!

            let _ = conn.shutdown().await.inspect_err(|error| {
                tracing::warn!("failed to disconnect from old socket: {error}")
            });

            anyhow::bail!(
                "socket path {} is currently listening, is the server already running?",
                path.display()
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Not found-- socket (probably) doesn't exist, so we can go ahead
            // and try to create it

            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            // Connection refused-- meaning the socket exists but is probably
            // from a dead server. We'll try to remove it first before binding
            // a fresh socket

            let result = tokio::fs::remove_file(path).await;
            match result {
                Ok(()) => {
                    tracing::debug!("cleaned up old dead socket");
                }
                Err(error) => {
                    tracing::warn!("failed to remove old dead socket: {error}");
                }
            }

            Ok(())
        }
        Err(error) => {
            // Another error. This has a good chance of failing, but we'll let
            // it fail when trying to bind the socket

            tracing::warn!(
                "encountered unexpected error when checking if server socket already exists: {error}"
            );
            Ok(())
        }
    }
}
