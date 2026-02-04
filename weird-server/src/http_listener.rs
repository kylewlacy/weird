use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade, ws},
    response::IntoResponse,
};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::sync::RwLock;

use crate::{
    message::{ClientMessage, ServerEvent, ServerMessage, SyncWorldRequest},
    world::World,
};

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<RwLock<World>>,
}

pub fn router(state: AppState) -> axum::Router {
    let router = axum::Router::new()
        .route("/ws", axum::routing::any(ws_endpoint_handler))
        .with_state(state);

    router
}

async fn ws_endpoint_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| ws_handler(state, socket).await)
}

async fn ws_handler(state: AppState, socket: ws::WebSocket) {
    tracing::info!("web client connected");

    // This oneshot channel will close when dropped, which will signal
    // to the sending side to exit
    let (_disconnect_tx, mut disconnect_rx) =
        tokio::sync::oneshot::channel::<std::convert::Infallible>();

    // Split the socket into a sending side and receiving side. The receiving
    // side is handled like normal in a loop, while the sending side uses
    // an MPSC queue in a separate Tokio task, so we can send messages
    // asynchronously and concurrently (e.g. subscribed events)
    let (socket_tx, mut socket_rx) = {
        let (mut socket_tx, socket_rx) = socket.split();
        let (send_channel_tx, mut send_channel_rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            loop {
                let message = tokio::select! {
                    message = send_channel_rx.recv() => {
                        let Some(message) = message else {
                            break;
                        };
                        message
                    }
                    _ = &mut disconnect_rx => {
                        // Socket disconnected
                        break;
                    }
                };

                let result = socket_tx.send(message).await;
                if result.is_err() {
                    // Failed to send message (socket probably disconnected)
                    break;
                }
            }
        });
        (send_channel_tx, socket_rx)
    };

    while let Some(message) = socket_rx.next().await {
        let message = match message {
            Err(error) => {
                tracing::warn!("received web client error: {error}");
                break;
            }
            Ok(ws::Message::Close(close_frame)) => {
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
            Ok(ws::Message::Ping(_) | ws::Message::Pong(_)) => {
                continue;
            }
            Ok(ws::Message::Text(ref text)) => text,
            Ok(ws::Message::Binary(ref bytes)) => {
                let text = str::from_utf8(&bytes);
                match text {
                    Ok(text) => text,
                    Err(error) => {
                        tracing::warn!("failed to decode web client binary: {error}");
                        break;
                    }
                }
            }
        };

        let message = facet_styx::from_str::<ClientMessage>(message);
        let message = match message {
            Ok(message) => message,
            Err(error) => {
                tracing::warn!("invalid message from web client: {error}");
                break;
            }
        };

        tracing::info!("got web client message: {message:?}");

        match message {
            ClientMessage::SyncWorld {
                sync_world: SyncWorldRequest { request_id },
            } => {
                let world = state.world.read().await;
                let event = world.initial_client_world_did_change_event();
                let mut events_rx = world.subscribe_to_world_did_change_events();
                let response = ServerMessage::Event {
                    id: request_id.clone(),
                    event: ServerEvent::WorldDidChange(event),
                };

                let styx = facet_styx::to_string(&response);
                let styx = match styx {
                    Ok(styx) => styx,
                    Err(error) => {
                        tracing::warn!("failed to serialize message: {error}");
                        continue;
                    }
                };
                drop(world);

                let result = socket_tx.send(ws::Message::text(styx)).await;
                match result {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::warn!("failed to send web response: {error}");
                    }
                }

                let socket_tx = socket_tx.clone();
                tokio::spawn(async move {
                    loop {
                        let event = events_rx.recv().await;
                        let event = match event {
                            Ok(event) => event,
                            Err(error) => {
                                tracing::warn!(
                                    "WorldChangeEvent subscription failed to receive: {error}"
                                );
                                break;
                            }
                        };
                        let response = ServerMessage::Event {
                            id: request_id.clone(),
                            event: ServerEvent::WorldDidChange(event),
                        };

                        let styx = facet_styx::to_string(&response);
                        let styx = match styx {
                            Ok(styx) => styx,
                            Err(error) => {
                                tracing::warn!("failed to serialize message: {error}");
                                break;
                            }
                        };

                        let result = socket_tx.send(ws::Message::text(styx)).await;
                        match result {
                            Ok(()) => {}
                            Err(error) => {
                                tracing::warn!("failed to send web response: {error}");
                            }
                        }
                    }
                });
            }
            ClientMessage::Render { .. } => {
                tracing::warn!("message not supported for web connections");
                continue;
            }
        }
    }
}
