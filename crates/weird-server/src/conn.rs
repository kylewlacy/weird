use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::Instrument;
use weird_core::{
    proto::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, Request, Response},
    world::{ROOT_NODE_ID, World},
};

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<RwLock<World>>,
}

pub async fn handle_conn(
    state: AppState,
    mut conn: weird_core::world::Connection,
    mut client_in: tokio::sync::mpsc::Receiver<JsonRpcRequest<Request>>,
    client_out: tokio::sync::mpsc::Sender<JsonRpcResponse<Response>>,
) {
    let mut window_node = None;

    while let Some(request) = client_in.recv().await {
        tracing::info!("got JSON RPC request: {request:?}");

        match request.body {
            Request::SyncWorld {} => {
                let world = state.world.read().await;

                let event = world.initial_client_world_did_change_event();
                let mut events_rx = world.subscribe_to_world_did_change_events();
                let response =
                    JsonRpcResponse::result(request.id.clone(), Response::WorldDidChange(event));

                drop(world);

                let out_result = client_out.send(response).await;
                match out_result {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::warn!("failed to send response: {error}");
                    }
                }

                let client_out = client_out.clone();
                tokio::spawn(
                    async move {
                        loop {
                            let event = events_rx.recv().await;
                            let event = match event {
                                Ok(event) => event,
                                Err(error) => {
                                    tracing::info!(
                                        "exiting because WorldChangeEvent subscription failed to receive: {error}"
                                    );
                                    break;
                                }
                            };
                            let response = JsonRpcResponse::result(
                                request.id.clone(),
                                Response::WorldDidChange(event),
                            );

                            let out_result = client_out.send(response).await;
                            match out_result {
                                Ok(()) => {}
                                Err(error) => {
                                    tracing::info!("exiting because WorldDidChange subscription failed to send: {error}");
                                    break;
                                }
                            }
                        }
                    }
                    .instrument(tracing::info_span!("sync_world")),
                );
            }
            Request::TriggerEvent(trigger_event) => {
                let mut world = state.world.write().await;

                let result = world.trigger_event(trigger_event).await.map_or_else(
                    |error| {
                        Err(JsonRpcError {
                            code: 2,
                            message: format!("trigger event error: {error:?}"),
                            data: serde_json::Value::Null,
                        })
                    },
                    |_| Ok(Response::Empty),
                );
                let response = JsonRpcResponse::new(request.id, result);

                let out_result = client_out.send(response).await;
                match out_result {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::warn!("failed to send response: {error}");
                    }
                }
            }
            Request::Render(render) => {
                let mut world = state.world.write().await;

                let response = if let Some(window_node) = window_node {
                    let result = world.set_node_children(window_node, render, conn.id);
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
                        |_| Ok(Response::Empty),
                    )
                } else {
                    window_node = Some(
                        world.append_node(
                            weird_core::world::Element::new("Window")
                                .children(render)
                                .into(),
                            ROOT_NODE_ID,
                            conn.id,
                        ),
                    );
                    Ok(Response::Empty)
                };
                let response = JsonRpcResponse::new(request.id, response);

                let out_result = client_out.send(response).await;
                match out_result {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::warn!("failed to send response: {error}");
                    }
                }
            }
            Request::NextEvent {} => {
                let event = conn.next_event().await;
                let response = event.map_or_else(|| Response::Empty, Response::Event);

                let response = JsonRpcResponse::result(request.id, response);
                let out_result = client_out.send(response).await;
                match out_result {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::warn!("failed to send response: {error}");
                    }
                }
            }
        }
    }
}
