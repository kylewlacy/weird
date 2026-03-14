use crate::world::{NodeTree, WorldDidChangeEvent};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcRequest<Body> {
    #[serde(rename = "jsonrpc")]
    _json_rpc: JsonRpcVersion,
    pub id: Option<JsonRpcRequestId>,
    #[serde(flatten)]
    pub body: Body,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<T> {
    Result(JsonRpcResponseResult<T>),
    Error(JsonRpcResponseError),
}

impl<T> JsonRpcResponse<T> {
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponseResult<T> {
    #[serde(rename = "jsonrpc")]
    _json_rpc: JsonRpcVersion,
    result: T,
    id: Option<JsonRpcRequestId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponseError {
    #[serde(rename = "jsonrpc")]
    _json_rpc: JsonRpcVersion,
    error: JsonRpcError,
    id: Option<JsonRpcRequestId>,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonRpcRequestId {
    String(String),
    Number(f64),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "camelCase")]
pub enum Request {
    #[serde(rename_all = "camelCase")]
    SyncWorld {},

    Render(Vec<NodeTree>),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ServerEvent {
    WorldDidChange(WorldDidChangeEvent),
}
