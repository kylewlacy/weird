use std::sync::Arc;

use tokio::{io::AsyncBufReadExt as _, sync::RwLock};

use crate::{
    message::ClientMessage,
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

    let mut frame_node = None;

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

        let message = facet_styx::from_str::<ClientMessage>(&line);
        let message = match message {
            Ok(message) => message,
            Err(error) => {
                tracing::warn!("invalid message from Unix client: {error}");
                break;
            }
        };

        tracing::info!("got unix client message: {message:?}");

        match message {
            ClientMessage::Show { show } => {
                let mut world = state.world.write().await;

                let frame_node = if let Some(frame_node) = frame_node {
                    frame_node
                } else {
                    let node = world.create_node(
                        crate::world::ElementTree::new(
                            "Frame",
                            crate::world::ElementProperties::default(),
                            vec![],
                        )
                        .into(),
                    );
                    let result = world.insert_node(InsertNode {
                        parent: ROOT_NODE_ID,
                        child: node,
                        offset: InsertNodeOffset::END,
                    });
                    if let Err(error) = result {
                        tracing::error!("failed to insert node in show message: {error:?}");
                        break;
                    }
                    *frame_node.insert(node)
                };

                let frame_children = world.node_children(frame_node).unwrap_or_default().to_vec();
                for child in frame_children {
                    let _ = world.remove_node(child).inspect_err(|error| {
                        tracing::warn!("failed to remove node in show message: {error:?}");
                    });
                }

                for node in show {
                    let node = world.create_node(node);
                    let _ = world
                        .insert_node(InsertNode {
                            parent: frame_node,
                            child: node,
                            offset: InsertNodeOffset::END,
                        })
                        .inspect_err(|error| {
                            tracing::warn!("failed to insert node in show message: {error:?}");
                        });
                }
            }
            ClientMessage::SyncWorld { .. } => {
                tracing::warn!("message not supported for Unix connections");
                continue;
            }
        }
    }

    Ok(())
}
