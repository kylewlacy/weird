use crate::world::{Event, Node, TriggerEvent, WorldDidChangeResponse};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcRequest<Body> {
    #[serde(rename = "jsonrpc")]
    _json_rpc: JsonRpcVersion,
    pub id: Option<JsonRpcRequestId>,
    #[serde(flatten)]
    pub body: Body,
}

impl<Body> JsonRpcRequest<Body> {
    pub fn new(id: Option<JsonRpcRequestId>, body: Body) -> Self {
        Self {
            _json_rpc: JsonRpcVersion::V2_0,
            id,
            body,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<T> {
    Result(JsonRpcResponseResult<T>),
    Error(JsonRpcResponseError),
}

impl<T> JsonRpcResponse<T> {
    pub fn new(id: Option<JsonRpcRequestId>, result: Result<T, JsonRpcError>) -> Self {
        match result {
            Ok(result) => Self::Result(JsonRpcResponseResult::new(id, result)),
            Err(error) => Self::Error(JsonRpcResponseError::new(id, error)),
        }
    }

    pub fn result(id: Option<JsonRpcRequestId>, result: T) -> Self {
        Self::Result(JsonRpcResponseResult {
            _json_rpc: JsonRpcVersion::V2_0,
            result,
            id,
        })
    }

    pub fn error(id: Option<JsonRpcRequestId>, error: JsonRpcError) -> Self {
        Self::Error(JsonRpcResponseError {
            _json_rpc: JsonRpcVersion::V2_0,
            error,
            id,
        })
    }

    pub fn id(&self) -> Option<&JsonRpcRequestId> {
        match self {
            Self::Result(result) => result.id.as_ref(),
            Self::Error(error) => error.id.as_ref(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponseResult<T> {
    #[serde(rename = "jsonrpc")]
    _json_rpc: JsonRpcVersion,
    id: Option<JsonRpcRequestId>,
    pub result: T,
}

impl<T> JsonRpcResponseResult<T> {
    pub fn new(id: Option<JsonRpcRequestId>, result: T) -> Self {
        Self {
            _json_rpc: JsonRpcVersion::V2_0,
            id,
            result,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponseError {
    #[serde(rename = "jsonrpc")]
    _json_rpc: JsonRpcVersion,
    id: Option<JsonRpcRequestId>,
    pub error: JsonRpcError,
}

impl JsonRpcResponseError {
    pub fn new(id: Option<JsonRpcRequestId>, error: JsonRpcError) -> Self {
        Self {
            _json_rpc: JsonRpcVersion::V2_0,
            id,
            error,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcError {
    pub code: u32,
    pub message: String,
    pub data: serde_json::Value,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum JsonRpcVersion {
    #[serde(rename = "2.0")]
    V2_0,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(untagged)]
pub enum JsonRpcRequestId {
    String(String),
    Number(i64),
}

impl From<&str> for JsonRpcRequestId {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for JsonRpcRequestId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i64> for JsonRpcRequestId {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "camelCase")]
pub enum Request {
    #[serde(rename_all = "camelCase")]
    SyncWorld {},

    #[serde(rename_all = "camelCase")]
    NextEvent {},

    TriggerEvent(TriggerEvent),

    Render(Vec<Node>),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Response {
    Empty,
    WorldDidChange(WorldDidChangeResponse),
    Event(Event),
}

impl Response {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::WorldDidChange(_) => "worldDidChange",
            Self::Event(_) => "event",
        }
    }
}
