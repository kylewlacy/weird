use std::{
    io::{BufRead as _, Write as _},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicI64},
};

use weird_core::{
    proto::{JsonRpcRequest, JsonRpcResponse},
    world::{InitRequest, InitResponse},
};

type RpcRequest = JsonRpcRequest<weird_core::proto::Request>;
type RpcResponse = JsonRpcResponse<weird_core::proto::Response>;

#[derive(Clone)]
pub struct WeirdClient {
    next_id: Arc<AtomicI64>,
    request_tx: crossbeam_channel::Sender<RpcRequest>,
    response_rx: crossbeam_channel::Receiver<RpcResponse>,
    _init_response: InitResponse,
}

impl WeirdClient {
    fn init(
        request_tx: crossbeam_channel::Sender<RpcRequest>,
        response_rx: crossbeam_channel::Receiver<RpcResponse>,
        init_request: InitRequest,
    ) -> Result<Self, WeirdClientError> {
        let next_id = Arc::new(AtomicI64::new(1));
        let init_id = next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Send "init" message
        request_tx
            .send(RpcRequest::new(
                Some(init_id.into()),
                weird_core::proto::Request::Init(init_request),
            ))
            .map_err(|_| WeirdClientError::ChannelClosed)?;

        // Get "init" response
        let response = response_rx
            .iter()
            .find(|response| response.id() == Some(&init_id.into()))
            .ok_or(WeirdClientError::ChannelClosed)?;
        let response = match response {
            JsonRpcResponse::Result(result) => result.result,
            JsonRpcResponse::Error(error) => {
                return Err(WeirdClientError::RpcError(error.error));
            }
        };
        let weird_core::proto::Response::Init(init_response) = response else {
            return Err(WeirdClientError::RpcUnexpectedResponse {
                expected: "init",
                actual: response.kind(),
            });
        };

        Ok(Self {
            _init_response: init_response,
            request_tx,
            response_rx,
            next_id,
        })
    }

    pub fn builder() -> WeirdClientBuilder {
        WeirdClientBuilder { app: None }
    }

    fn request(
        &self,
        request: weird_core::proto::Request,
    ) -> Result<weird_core::proto::Response, WeirdClientError> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let response_rx = self.response_rx.clone();
        self.request_tx
            .send(RpcRequest::new(Some(id.into()), request))
            .map_err(|_| WeirdClientError::ChannelClosed)?;

        let response = response_rx
            .iter()
            .find(|response| response.id() == Some(&id.into()))
            .ok_or(WeirdClientError::ChannelClosed)?;

        match response {
            JsonRpcResponse::Result(result) => Ok(result.result),
            JsonRpcResponse::Error(error) => Err(WeirdClientError::RpcError(error.error)),
        }
    }

    pub fn try_render(
        &self,
        nodes: impl IntoIterator<Item = weird_core::world::Node>,
    ) -> Result<(), WeirdClientError> {
        let nodes = nodes.into_iter().collect();
        let _response = self.request(weird_core::proto::Request::Render(nodes))?;
        Ok(())
    }

    pub fn render(&self, nodes: impl IntoIterator<Item = weird_core::world::Node>) {
        let result = self.try_render(nodes);
        match result {
            Ok(()) => {}
            Err(error @ WeirdClientError::RpcError(_)) => {
                // TODO: Use something other than eprintln! for error reporting
                eprintln!("failed to render: {error}");
            }
            Err(error) => {
                panic!("error during render: {error}");
            }
        }
    }

    pub fn next_event(&self) -> Result<Option<weird_core::world::Event>, WeirdClientError> {
        let response = self.request(weird_core::proto::Request::NextEvent {});
        match response {
            Ok(weird_core::proto::Response::Event(event)) => Ok(Some(event)),
            Ok(weird_core::proto::Response::Empty) | Err(WeirdClientError::ChannelClosed) => {
                Ok(None)
            }
            Ok(response) => Err(WeirdClientError::RpcUnexpectedResponse {
                expected: "event",
                actual: response.kind(),
            }),
            Err(error) => Err(error),
        }
    }
}

pub struct WeirdClientBuilder {
    app: Option<String>,
}

impl WeirdClientBuilder {
    pub fn app(mut self, app: &str) -> Self {
        self.app = Some(app.to_string());
        self
    }

    pub fn connect(self) -> Result<WeirdClient, WeirdClientError> {
        let socket_path = if let Some(socket_path) = std::env::var_os("WEIRD_SOCKET") {
            PathBuf::from(socket_path)
        } else {
            let runtime_dir =
                std::env::var_os("XDG_RUNTIME_DIR").ok_or(WeirdClientError::EnvVarNotSet {
                    env_var: "XDG_RUNTIME_DIR",
                })?;
            Path::new(&runtime_dir).join("weird.sock")
        };
        let stream = UnixStream::connect(&socket_path)?;
        let read_stream = stream.try_clone()?;
        let read_stream = std::io::BufReader::new(read_stream);
        let write_stream = stream;

        let (request_tx, request_rx) = crossbeam_channel::unbounded();
        let (response_tx, response_rx) = crossbeam_channel::unbounded();

        // TODO: Use some other way of error reporting beside eprintln!
        std::thread::spawn(move || {
            for line in read_stream.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        eprintln!("error reading from weird socket: {error:#}");
                        continue;
                    }
                };
                let message =
                    serde_json::from_str::<JsonRpcResponse<weird_core::proto::Response>>(&line);
                let message = match message {
                    Ok(message) => message,
                    Err(error) => {
                        eprintln!("error deserializing line from weird socket: {error:#}");
                        continue;
                    }
                };
                let send_result = response_tx.send(message);
                if send_result.is_err() {
                    break;
                }
            }
        });
        std::thread::spawn(move || {
            while let Ok(message) = request_rx.recv() {
                let result = serde_json::to_writer(&write_stream, &message);
                if let Err(error) = result {
                    eprintln!("error writing to weird socket: {error:#}");
                }

                let result = writeln!(&write_stream);
                if let Err(error) = result {
                    eprintln!("error writing to weird socket: {error:#}");
                }
            }
        });
        let init_request = InitRequest {
            client: Some("weird-client".to_string()),
            app: self.app,
            ..Default::default()
        };
        let client = WeirdClient::init(request_tx, response_rx, init_request)?;
        Ok(client)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WeirdClientError {
    #[error("env var not set: ${env_var}")]
    EnvVarNotSet { env_var: &'static str },

    #[error("channel for client closed unexpectedly")]
    ChannelClosed,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("received RPC error code {} from server: {}", _0.code, _0.message)]
    RpcError(weird_core::proto::JsonRpcError),

    #[error("received unexpected RPC response from server: expected {expected} but got {actual}")]
    RpcUnexpectedResponse {
        expected: &'static str,
        actual: &'static str,
    },
}
