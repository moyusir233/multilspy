use multilspy_protocol::protocol::common::WorkspaceSymbol;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
}

pub const ERR_INTERNAL: i32 = -32603;
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERR_INVALID_PARAMS: i32 = -32602;
pub const ERR_LSP_FAILED: i32 = -32000;

impl IpcResponse {
    pub fn success(result: serde_json::Value) -> Self {
        Self {
            result: Some(result),
            error: None,
        }
    }

    pub fn error(code: i32, message: String) -> Self {
        Self {
            result: None,
            error: Some(IpcError { code, message }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionParams {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencesIpcParams {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub include_declaration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbolsIpcParams {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveCallsIpcParams {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbolsIpcParams {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbolResolveIpcParams {
    pub symbol: WorkspaceSymbol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub workspace: String,
    pub pid: u32,
    pub port: u16,
    pub uptime_secs: u64,
}

pub async fn send_request(port: u16, request: &IpcRequest) -> anyhow::Result<IpcResponse> {
    let url = format!("http://127.0.0.1:{}/rpc", port);
    let client = reqwest::Client::new();
    let resp = client.post(&url).json(request).send().await?;
    let ipc_resp: IpcResponse = resp.json().await?;
    Ok(ipc_resp)
}

pub async fn ping(port: u16) -> bool {
    let req = IpcRequest {
        method: "ping".to_string(),
        params: serde_json::json!(null),
    };
    match send_request(port, &req).await {
        Ok(r) => r.error.is_none(),
        Err(_) => false,
    }
}
