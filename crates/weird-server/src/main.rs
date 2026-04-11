use std::path::Path;

use anyhow::Context as _;
use tokio::io::AsyncWriteExt as _;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::conn::AppState;

mod conn;
mod http_listener;
mod unix_listener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_TRACING_DIRECTIVE: &str =
        concat!(env!("CARGO_CRATE_NAME"), "=info,weird_core=info,warn");
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_TRACING_DIRECTIVE)),
        )
        .init();

    let state = AppState {
        world: weird_core::world::World::default(),
    };

    let http_app = http_listener::router(state.clone());

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
    let unix_socket_fut = unix_listener::serve_unix_socket(unix_socket, state);

    tokio::try_join!(http_server_fut, unix_socket_fut)?;

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
