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
            ClientMessage::Render { render } => {
                let mut world = state.world.write().await;

                if let Some(frame_node) = frame_node {
                    let result = world.set_node_children(frame_node, render);
                    if let Err(error) = result {
                        tracing::error!("failed to update node in render message: {error:?}");
                        break;
                    }
                } else {
                    let frame_node = frame_node.insert(
                        world.create_node(
                            crate::world::ElementTree::new(
                                "Frame",
                                crate::world::ElementProperties::default(),
                                render,
                            )
                            .into(),
                        ),
                    );
                    let result = world.insert_node(InsertNode {
                        parent: ROOT_NODE_ID,
                        child: *frame_node,
                        offset: InsertNodeOffset::END,
                    });
                    if let Err(error) = result {
                        tracing::error!("failed to insert node in render message: {error:?}");
                        break;
                    }
                };
            }
            ClientMessage::SyncWorld { .. } => {
                tracing::warn!("message not supported for Unix connections");
                continue;
            }
        }
    }

    Ok(())
}
