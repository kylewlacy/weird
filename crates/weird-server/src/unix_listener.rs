use std::sync::Arc;

use tokio::{io::AsyncBufReadExt as _, sync::RwLock};

use crate::{
    protocol::{JsonRpcRequest, Request},
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

    let (rx, _tx) = conn.split();

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

        tracing::info!("got unix client message: {request:?}");

        match request.body {
            Request::Render(render) => {
                let mut world = state.world.write().await;

                if let Some(window_node) = window_node {
                    let result = world.set_node_children(window_node, render);
                    if let Err(error) = result {
                        tracing::error!("failed to update node in render message: {error:?}");
                        break;
                    }
                } else {
                    let window_node = window_node.insert(
                        world.create_node(
                            crate::world::ElementTree::new("Window")
                                .children(render)
                                .into(),
                        ),
                    );
                    let result = world.insert_node(InsertNode {
                        parent: ROOT_NODE_ID,
                        child: *window_node,
                        offset: InsertNodeOffset::END,
                    });
                    if let Err(error) = result {
                        tracing::error!("failed to insert node in render request: {error:?}");
                        break;
                    }
                };
            }
            Request::SyncWorld { .. } => {
                tracing::warn!("message not supported for Unix connections");
                continue;
            }
        }
    }

    Ok(())
}
