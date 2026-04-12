//! JSON-RPC 2.0 message types for the Language Server Protocol.
//!
//! This module defines the base protocol message structures that form the foundation of
//! LSP communication, as specified in the
//! [Base Protocol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#baseProtocol)
//! section of the LSP 3.17 specification.
//!
//! # Structures
//!
//! | Structure | LSP Spec Section |
//! |-----------|-----------------|
//! | [`RequestId`] | [Request ID](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#requestMessage) — `integer \| string` |
//! | [`Request`] | [RequestMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#requestMessage) |
//! | [`Response`] | [ResponseMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#responseMessage) |
//! | [`ResponseResult`] | Wraps the mutually exclusive `result` / `error` fields of `ResponseMessage` |
//! | [`ResponseError`] | [ResponseError](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#responseError) |
//! | [`Notification`] | [NotificationMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#notificationMessage) |

use crate::error::ErrorCodes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

const JSONRPC_VERSION: &str = "2.0";

/// A request identifier that can be either an integer or a string.
///
/// Per the JSON-RPC 2.0 specification and LSP, request IDs are used to correlate
/// requests with their responses. The protocol allows both integer and string identifiers.
///
/// # Wire Format
///
/// Either a JSON number or a JSON string:
/// ```json
/// 42
/// ```
/// or
/// ```json
/// "abc-123"
/// ```
///
/// # LSP Specification
///
/// See [RequestMessage.id](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#requestMessage).
/// LSP type: `integer | string`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric request ID.
    Number(u64),
    /// String request ID.
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

/// A JSON-RPC request message.
///
/// Every processed request must send a response back to the sender.
///
/// # Wire Format
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 1,
///   "method": "textDocument/definition",
///   "params": { "textDocument": { "uri": "file:///test.rs" }, "position": { "line": 0, "character": 0 } }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `jsonrpc` | `String` | Yes | JSON-RPC protocol version. Always `"2.0"`. |
/// | `id` | [`RequestId`] | Yes | The request id. Used to match responses. LSP type: `integer \| string`. |
/// | `method` | `String` | Yes | The method to be invoked. |
/// | `params` | `Option<Value>` | No | The method's params. LSP type: `array \| object`. |
///
/// # LSP Specification
///
/// See [RequestMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#requestMessage).
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC protocol version. Always `"2.0"`.
    pub jsonrpc: String,
    /// The request id. Used to correlate the request with its response.
    pub id: RequestId,
    /// The method to be invoked (e.g., `"textDocument/definition"`).
    pub method: String,
    /// The method's params.
    #[serde(default)]
    pub params: Option<Value>,
}

/// A JSON-RPC response message.
///
/// Sent as a result of a request. A response always contains either a `result` or an `error`,
/// but not both. The `result` field wraps both cases via [`ResponseResult`].
///
/// # Wire Format (success)
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 1,
///   "result": [{ "uri": "file:///test.rs", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 10 } } }]
/// }
/// ```
///
/// # Wire Format (error)
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 1,
///   "error": { "code": -32601, "message": "Method not found" }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `jsonrpc` | `String` | Yes | JSON-RPC protocol version. Always `"2.0"`. |
/// | `id` | [`RequestId`] | Yes | The request id this response corresponds to. LSP type: `integer \| string \| null`. |
/// | `result` | `Option<ResponseResult>` | No | Flattened: either a `result` value or an `error` object. `None` when both fields are absent. |
///
/// # LSP Specification
///
/// See [ResponseMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#responseMessage).
///
/// **Note:** Per the spec, `id` can also be `null` when the request id could not be determined
/// (e.g., parse errors). This implementation uses [`RequestId`] which does not model `null`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    /// JSON-RPC protocol version. Always `"2.0"`.
    pub jsonrpc: String,
    /// The request id this response corresponds to.
    pub id: RequestId,
    /// The response payload — either a successful result or an error.
    /// `None` when neither `result` nor `error` is present in the message.
    #[serde(flatten, default)]
    pub result: Option<ResponseResult>,
}

/// Wraps the mutually exclusive `result` and `error` fields of a [`Response`].
///
/// Per the JSON-RPC 2.0 specification:
/// - On success: `result` is REQUIRED and `error` MUST NOT exist.
/// - On failure: `error` is REQUIRED and `result` MUST NOT exist.
///
/// This enum is serialized with `#[serde(rename_all = "camelCase")]` so that the
/// `Result` variant maps to the `"result"` JSON key and `Error` maps to `"error"`.
///
/// # LSP Specification
///
/// See [ResponseMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#responseMessage).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseResult {
    /// The successful result value. LSP type: `LSPAny`.
    Result(Value),
    /// The error object in case a request fails.
    Error(ResponseError),
}

/// A structured error returned in a [`Response`] when a request fails.
///
/// # Wire Format
///
/// ```json
/// {
///   "code": -32601,
///   "message": "Method not found",
///   "data": { "detail": "textDocument/fooBar is not supported" }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `code` | [`ErrorCodes`] | Yes | A number indicating the error type that occurred. See [ErrorCodes](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes). |
/// | `message` | `String` | Yes | A short description of the error. |
/// | `data` | `Option<Value>` | No | A primitive or structured value with additional information about the error. LSP type: `LSPAny`. |
///
/// # LSP Specification
///
/// See [ResponseError](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#responseError).
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    /// A number indicating the error type that occurred.
    pub code: ErrorCodes,
    /// A string providing a short description of the error.
    pub message: String,
    /// A primitive or structured value that contains additional information about the error.
    #[serde(default)]
    pub data: Option<Value>,
}

/// A JSON-RPC notification message.
///
/// A processed notification message must not send a response back. Notifications work like events.
///
/// # Wire Format
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "method": "textDocument/didOpen",
///   "params": { "textDocument": { "uri": "file:///test.rs", "languageId": "rust", "version": 1, "text": "" } }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `jsonrpc` | `String` | Yes | JSON-RPC protocol version. Always `"2.0"`. |
/// | `method` | `String` | Yes | The method to be invoked. |
/// | `params` | `Option<Value>` | No | The notification's params. LSP type: `array \| object`. |
///
/// # LSP Specification
///
/// See [NotificationMessage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#notificationMessage).
#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    /// JSON-RPC protocol version. Always `"2.0"`.
    pub jsonrpc: String,
    /// The notification method (e.g., `"textDocument/didOpen"`, `"initialized"`, `"exit"`).
    pub method: String,
    /// The notification's params.
    #[serde(default)]
    pub params: Option<Value>,
}

impl Request {
    pub fn new(id: RequestId, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method,
            params,
        }
    }
}

impl Response {
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(ResponseResult::Result(result)),
        }
    }

    pub fn error(id: RequestId, code: ErrorCodes, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(ResponseResult::Error(ResponseError {
                code,
                message,
                data,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization_with_number_id() {
        let request = Request::new(
            RequestId::Number(1),
            "textDocument/definition".to_string(),
            Some(
                json!({ "textDocument": { "uri": "file:///test.rs" }, "position": { "line": 0, "character": 0 } }),
            ),
        );

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(1));
        assert_eq!(deserialized.method, "textDocument/definition");
        assert!(deserialized.params.is_some());
    }

    #[test]
    fn test_request_serialization_with_string_id() {
        let request = Request::new(
            RequestId::String("abc-123".to_string()),
            "textDocument/hover".to_string(),
            Some(json!({ "position": { "line": 5, "character": 10 } })),
        );

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::String("abc-123".to_string()));
        assert_eq!(deserialized.method, "textDocument/hover");
    }

    #[test]
    fn test_request_serialization_without_params() {
        let request = Request::new(RequestId::Number(99), "shutdown".to_string(), None);

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(99));
        assert_eq!(deserialized.method, "shutdown");
        assert!(deserialized.params.is_none());
    }

    #[test]
    fn test_request_deserialization_from_raw_json() {
        let raw = r#"{"jsonrpc":"2.0","id":42,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///main.rs"},"position":{"line":10,"character":5}}}"#;
        let request: Request = serde_json::from_str(raw).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::Number(42));
        assert_eq!(request.method, "textDocument/completion");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_request_deserialization_without_params_field() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();

        assert_eq!(request.id, RequestId::Number(1));
        assert_eq!(request.method, "shutdown");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_response_success_serialization() {
        let response = Response::success(
            RequestId::Number(1),
            json!([{ "uri": "file:///test.rs", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 10 } } }]),
        );

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(1));
        assert!(matches!(
            deserialized.result,
            Some(ResponseResult::Result(_))
        ));
    }

    #[test]
    fn test_response_success_with_null_result() {
        let response = Response::success(RequestId::Number(3), Value::Null);

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(3));
        if let Some(ResponseResult::Result(val)) = deserialized.result {
            assert!(val.is_null());
        } else {
            panic!("expected Some(Result(null))");
        }
    }

    #[test]
    fn test_response_error_with_data() {
        let response = Response::error(
            RequestId::Number(2),
            ErrorCodes::MethodNotFound,
            "Method not found".to_string(),
            Some(json!({ "detail": "textDocument/fooBar is not supported" })),
        );

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::Number(2));
        match deserialized.result {
            Some(ResponseResult::Error(err)) => {
                assert_eq!(err.code, ErrorCodes::MethodNotFound);
                assert_eq!(err.message, "Method not found");
                assert!(err.data.is_some());
            }
            other => panic!("expected Some(Error(...)), got {:?}", other),
        }
    }

    #[test]
    fn test_response_error_without_data() {
        let response = Response::error(
            RequestId::String("req-7".to_string()),
            ErrorCodes::InternalError,
            "Internal error".to_string(),
            None,
        );

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, RequestId::String("req-7".to_string()));
        match deserialized.result {
            Some(ResponseResult::Error(err)) => {
                assert_eq!(err.code, ErrorCodes::InternalError);
                assert_eq!(err.message, "Internal error");
                assert!(err.data.is_none());
            }
            other => panic!("expected Some(Error(...)), got {:?}", other),
        }
    }

    #[test]
    fn test_response_without_result_and_error() {
        let raw = r#"{"jsonrpc":"2.0","id":10}"#;
        let response: Response = serde_json::from_str(raw).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, RequestId::Number(10));
        assert!(response.result.is_none());
    }

    #[test]
    fn test_response_error_deserialization_from_raw_json() {
        let raw =
            r#"{"jsonrpc":"2.0","id":5,"error":{"code":-32601,"message":"Method not found"}}"#;
        let response: Response = serde_json::from_str(raw).unwrap();

        assert_eq!(response.id, RequestId::Number(5));
        match response.result {
            Some(ResponseResult::Error(err)) => {
                assert_eq!(err.code, ErrorCodes::MethodNotFound);
                assert_eq!(err.message, "Method not found");
                assert!(err.data.is_none());
            }
            other => panic!("expected Some(Error(...)), got {:?}", other),
        }
    }

    #[test]
    fn test_response_error_code_roundtrip_all_variants() {
        let codes = vec![
            ErrorCodes::ParseError,
            ErrorCodes::InvalidRequest,
            ErrorCodes::MethodNotFound,
            ErrorCodes::InvalidParams,
            ErrorCodes::InternalError,
            ErrorCodes::ServerNotInitialized,
            ErrorCodes::UnknownErrorCode,
            ErrorCodes::RequestFailed,
            ErrorCodes::ServerCancelled,
            ErrorCodes::ContentModified,
            ErrorCodes::RequestCancelled,
        ];

        for code in codes {
            let response = Response::error(RequestId::Number(1), code, "test".to_string(), None);

            let serialized = serde_json::to_string(&response).unwrap();
            let deserialized: Response = serde_json::from_str(&serialized).unwrap();

            match deserialized.result {
                Some(ResponseResult::Error(err)) => {
                    assert_eq!(err.code, code, "roundtrip failed for {:?}", code);
                }
                other => panic!("expected Some(Error(...)) for {:?}, got {:?}", code, other),
            }
        }
    }

    #[test]
    fn test_response_error_with_string_id() {
        let raw = r#"{"jsonrpc":"2.0","id":"uuid-abc","error":{"code":-32700,"message":"Parse error","data":"unexpected token"}}"#;
        let response: Response = serde_json::from_str(raw).unwrap();

        assert_eq!(response.id, RequestId::String("uuid-abc".to_string()));
        match response.result {
            Some(ResponseResult::Error(err)) => {
                assert_eq!(err.code, ErrorCodes::ParseError);
                assert_eq!(err.data, Some(json!("unexpected token")));
            }
            other => panic!("expected Some(Error(...)), got {:?}", other),
        }
    }

    #[test]
    fn test_notification_serialization_with_params() {
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "textDocument/didOpen".to_string(),
            params: Some(
                json!({ "textDocument": { "uri": "file:///test.rs", "languageId": "rust", "version": 1, "text": "" } }),
            ),
        };

        let serialized = serde_json::to_string(&notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.jsonrpc, "2.0");
        assert_eq!(deserialized.method, "textDocument/didOpen");
        assert!(deserialized.params.is_some());
    }

    #[test]
    fn test_notification_serialization_without_params() {
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "exit".to_string(),
            params: None,
        };

        let serialized = serde_json::to_string(&notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.method, "exit");
        assert!(deserialized.params.is_none());
    }

    #[test]
    fn test_notification_deserialization_from_raw_json() {
        let raw = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
        let notification: Notification = serde_json::from_str(raw).unwrap();

        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "initialized");
        assert!(notification.params.is_some());
    }

    #[test]
    fn test_notification_deserialization_without_params_field() {
        let raw = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        let notification: Notification = serde_json::from_str(raw).unwrap();

        assert_eq!(notification.method, "exit");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_request_id_display() {
        assert_eq!(RequestId::Number(42).to_string(), "42");
        assert_eq!(RequestId::String("abc".to_string()).to_string(), "abc");
    }

    #[test]
    fn test_request_id_equality() {
        assert_eq!(RequestId::Number(1), RequestId::Number(1));
        assert_ne!(RequestId::Number(1), RequestId::Number(2));
        assert_eq!(
            RequestId::String("a".to_string()),
            RequestId::String("a".to_string())
        );
        assert_ne!(RequestId::Number(1), RequestId::String("1".to_string()));
    }
}
