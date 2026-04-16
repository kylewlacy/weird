use axum::{
    extract::{State, WebSocketUpgrade, ws},
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::{SinkExt as _, StreamExt as _};
use tracing::Instrument as _;

use weird_core::proto::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, Request};

use crate::conn::{AppState, handle_conn};

pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/ws", axum::routing::any(ws_endpoint_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            allowed_origin_middleware,
        ))
        .with_state(state)
}

async fn ws_endpoint_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| ws_handler(state, socket).await)
}

async fn ws_handler(state: AppState, socket: ws::WebSocket) {
    let (mut socket_tx, mut socket_rx) = socket.split();
    let (client_in_tx, client_in_rx) = tokio::sync::mpsc::channel(1);
    let (client_out_tx, mut client_out_rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(
        handle_conn(
            state,
            weird_core::world::ConnectionSource::Websocket {},
            client_in_rx,
            client_out_tx,
        )
        .instrument(tracing::info_span!("ws_handler", conn.kind = "websocket")),
    );

    loop {
        tokio::select! {
            request = socket_rx.next() => {
                let Some(request) = request else {
                    tracing::info!("web client closed connection (no close message)");
                    break;
                };
                let request = match request {
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
                        let text = str::from_utf8(bytes);
                        match text {
                            Ok(text) => text,
                            Err(error) => {
                                tracing::warn!("failed to decode web client binary: {error}");
                                break;
                            }
                        }
                    }
                };

                let request = serde_json::from_str::<JsonRpcRequest<Request>>(request);
                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        tracing::warn!("invalid JSON RPC request from web client: {error}");

                        let response = JsonRpcResponse::<()>::error(
                            None,
                            JsonRpcError {
                                code: 1,
                                message: format!("received invalid JSON RPC request: {error}"),
                                data: serde_json::Value::Null,
                            },
                        );
                        let json = serde_json::to_string(&response)
                            .expect("failed to serialize JSON RPC response");

                        let result = socket_tx.send(ws::Message::Text(json.into())).await;
                        match result {
                            Ok(()) => {}
                            Err(error) => {
                                tracing::warn!("failed to send web response: {error}");
                            }
                        }
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

                let json = serde_json::to_string(&response);
                let json = match json {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!("failed to serialize message: {error}");
                        break;
                    }
                };

                let result = socket_tx.send(ws::Message::text(json)).await;
                match result {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::warn!("failed to send web JSON RPC response: {error}");
                    }
                }
            }
        };
    }
}

#[axum::debug_middleware]
async fn allowed_origin_middleware(
    state: State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(origin) = request.headers().get("Origin") else {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(JsonError::new("'Origin' header not set")),
        )
            .into_response();
    };
    let origin = match origin.to_str() {
        Ok(origin) => origin,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(JsonError::new(format!(
                    "invalid value for 'Origin' header: {error}"
                ))),
            )
                .into_response();
        }
    };

    if !state.config.allow_origins.contains(origin) {
        tracing::info!(request_origin = origin, allowed_origins = ?state.config.allow_origins, "received request with disallowed origin header");
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(JsonError::new(format!("origin not allowed: {origin}"))),
        )
            .into_response();
    }

    next.run(request).await
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonError {
    error: String,
}

impl JsonError {
    fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}
