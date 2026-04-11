use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

pub struct ApiResponseWithStatus(pub StatusCode, pub ApiResponse);

impl IntoResponse for ApiResponseWithStatus {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(self.1)).into_response()
    }
}

#[derive(Deserialize)]
pub struct StartRequest {
    pub project_path: String,
}

#[derive(Deserialize)]
pub struct PositionRequest {
    pub project_path: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Deserialize)]
pub struct ReferencesRequest {
    pub project_path: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Deserialize)]
pub struct DocumentSymbolsRequest {
    pub project_path: String,
    pub file_path: String,
}

#[derive(Deserialize)]
pub struct RecursiveRequest {
    pub project_path: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub max_depth: Option<usize>,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ApiResponse {
    pub fn ok<T: Serialize>(result: T) -> Self {
        Self {
            status: "ok".to_string(),
            result: Some(serde_json::to_value(result).unwrap()),
            message: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            status: "error".to_string(),
            result: None,
            message: Some(message),
        }
    }

    pub fn already_running() -> Self {
        Self {
            status: "already_running".to_string(),
            result: None,
            message: None,
        }
    }

    pub fn stopped() -> Self {
        Self {
            status: "ok".to_string(),
            result: None,
            message: None,
        }
    }
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}
