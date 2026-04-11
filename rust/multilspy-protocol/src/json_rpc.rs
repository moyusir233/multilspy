use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(flatten)]
    pub result: ResponseResult,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseResult {
    Result(Value),
    Error(ResponseError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl Request {
    pub fn new(id: RequestId, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }
}

impl Response {
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: ResponseResult::Result(result),
        }
    }

    pub fn error(id: RequestId, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: ResponseResult::Error(ResponseError {
                code,
                message,
                data,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let request = Request::new(
            RequestId::Number(1),
            "textDocument/definition".to_string(),
            Some(json!({ "textDocument": { "uri": "file:///test.rs" }, "position": { "line": 0, "character": 0 } }))
        );

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(1));
        assert_eq!(deserialized.method, "textDocument/definition");
    }

    #[test]
    fn test_response_serialization() {
        let response = Response::success(
            RequestId::Number(1),
            json!([{ "uri": "file:///test.rs", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 10 } } }])
        );

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(1));
        assert!(matches!(deserialized.result, ResponseResult::Result(_)));
    }
}
