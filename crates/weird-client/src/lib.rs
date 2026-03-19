use std::{
    io::{BufRead as _, Write as _},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicI64},
};

use weird_core::proto::{JsonRpcRequest, JsonRpcResponse};

type RpcRequest = JsonRpcRequest<weird_core::proto::Request>;
type RpcResponse = JsonRpcResponse<weird_core::proto::Response>;

#[derive(Clone)]
pub struct WeirdClient {
    next_id: Arc<AtomicI64>,
    request_tx: crossbeam_channel::Sender<RpcRequest>,
    response_rx: crossbeam_channel::Receiver<RpcResponse>,
}

impl WeirdClient {
    pub fn connect() -> Result<Self, WeirdClientError> {
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
        Ok(Self {
            next_id: Arc::new(AtomicI64::new(1)),
            request_tx,
            response_rx,
        })
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
}
