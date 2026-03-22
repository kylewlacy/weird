use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _},
    sync::RwLock,
};

use weird_core::{
    proto::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, Request, Response},
    world::{InsertNode, InsertNodeOffset, ROOT_NODE_ID, World},
};

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

    let (rx, mut tx) = conn.split();

    let world_conn = {
        let mut world = state.world.write().await;
        world.create_connection()
    };
    let connection_id = world_conn.id;
    let mut window_node = None;

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

        let request = serde_json::from_str::<JsonRpcRequest<Request>>(&line);
        let request = match request {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!("invalid JSON RPC request from Unix client: {error}");
                break;
            }
        };

        tracing::info!(%connection_id, "got unix client message: {request:?}");

        match request.body {
            Request::Render(render) => {
                let mut world = state.world.write().await;

                let response = if let Some(window_node) = window_node {
                    let result = world.set_node_children(window_node, render, connection_id);
                    result.map_or_else(
                        |error| {
                            Err(JsonRpcError {
                                code: 1,
                                message: format!(
                                    "failed to update node in render request: {error:#?}"
                                ),
                                data: serde_json::Value::Null,
                            })
                        },
                        |()| Ok(Response::Empty),
                    )
                } else {
                    let window_node = window_node.insert(
                        world.create_node(
                            weird_core::world::Element::new("Window")
                                .children(render)
                                .into(),
                            connection_id,
                        ),
                    );
                    let result = world.insert_node(InsertNode {
                        parent: ROOT_NODE_ID,
                        child: *window_node,
                        offset: InsertNodeOffset::END,
                    });
                    result.map_or_else(
                        |error| {
                            Err(JsonRpcError {
                                code: 1,
                                message: format!(
                                    "failed to insert node in render request: {error:?}"
                                ),
                                data: serde_json::Value::Null,
                            })
                        },
                        |_| Ok(Response::Empty),
                    )
                };
                let response = JsonRpcResponse::new(request.id, response);
                let mut response_json = serde_json::to_string(&response)?;
                response_json.push('\n');

                let result = tx.write_all(response_json.as_bytes()).await;
                if let Err(error) = result {
                    tracing::warn!("failed to write response to Unix connection: {error:?}");
                };
            }
            Request::NextEvent {} => {
                let world = state.world.read().await;

                let response = world
                    .take_next_event(world_conn.id)
                    .await
                    .map_or_else(|| Response::Empty, Response::Event);

                let response = JsonRpcResponse::result(request.id, response);
                let mut response_json = serde_json::to_string(&response)?;
                response_json.push('\n');

                let result = tx.write_all(response_json.as_bytes()).await;
                if let Err(error) = result {
                    tracing::warn!("failed to write response to Unix connection: {error:?}");
                };
            }
            Request::SyncWorld { .. } | Request::TriggerEvent(_) => {
                tracing::warn!("message not supported for Unix connections");
                continue;
            }
        }
    }

    Ok(())
}
