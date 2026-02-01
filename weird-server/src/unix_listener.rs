use std::sync::Arc;

use tokio::{io::AsyncBufReadExt as _, sync::RwLock};

use crate::world::World;

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<RwLock<World>>,
}

pub async fn serve_unix_socket(
    socket: tokio::net::UnixListener,
    state: AppState,
) -> anyhow::Result<()> {
    loop {
        let (conn, _addr) = socket.accept().await?;
        tokio::spawn(handle_unix_conn(conn, state.clone()));
    }
}

async fn handle_unix_conn(mut conn: tokio::net::UnixStream, state: AppState) -> anyhow::Result<()> {
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
