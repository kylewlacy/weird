use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

use tracing::Instrument as _;
use weird_core::proto::{JsonRpcRequest, Request};

use crate::conn::{AppState, handle_conn};

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
    let world_conn = state.world.create_connection().await;
    let connection_id = world_conn.id;

    let (stream_rx, mut stream_tx) = conn.split();
    let (client_in_tx, client_in_rx) = tokio::sync::mpsc::channel(1);
    let (client_out_tx, mut client_out_rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(
        handle_conn(state, world_conn, client_in_rx, client_out_tx).instrument(
            tracing::info_span!(
                "connection",
                conn.id = %connection_id,
                conn.kind = "websocket"
            ),
        ),
    );

    let stream_rx = tokio::io::BufReader::new(stream_rx);
    let mut stream_rx_lines = stream_rx.lines();
    loop {
        tokio::select! {
            line = stream_rx_lines.next_line() => {
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

                let request = serde_json::from_str::<JsonRpcRequest<Request>>(&line);
                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        tracing::warn!("invalid JSON RPC request from Unix client: {error}");
                        break;
                    }
                };

                let in_result = client_in_tx.send(request).await;
                if let Err(error) = in_result {
                    tracing::info!("client connection unavailable: {error:?}");
                    break;
                }

            }
            response = client_out_rx.recv() => {
                let Some(response) = response else {
                    tracing::warn!("output channel closed for client");
                    break;
                };

                let mut response_json = serde_json::to_string(&response)?;
                response_json.push('\n');

                let result = stream_tx.write_all(response_json.as_bytes()).await;
                if let Err(error) = result {
                    tracing::warn!("failed to write response to Unix connection: {error:?}");
                };
            }
        }
    }

    Ok(())
}
