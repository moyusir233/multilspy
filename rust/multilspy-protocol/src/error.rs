//! Error types for the LSP protocol implementation.
//!
//! This module defines:
//! - [`ProtocolError`]: Application-level errors for transport and message handling.
//! - [`ErrorCodes`]: LSP 3.17 error codes as defined in the
//!   [ErrorCodes](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes)
//!   section of the specification.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use thiserror::Error;

/// Application-level errors for LSP transport and message handling.
///
/// These are **not** LSP protocol error codes — they represent errors that occur
/// in the Rust implementation layer when sending/receiving messages.
#[derive(Error, Debug)]
pub enum ProtocolError {
    /// JSON serialization or deserialization failed.
    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// An I/O error occurred during transport communication.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// The received message does not conform to the expected format.
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// The response ID does not match the request ID.
    #[error("Request ID mismatch")]
    RequestIdMismatch,

    /// The transport connection has been closed.
    #[error("Transport closed")]
    TransportClosed,
}

/// LSP 3.17 error codes used in [`ResponseError.code`](crate::json_rpc::ResponseError).
///
/// These codes are divided into three ranges:
///
/// 1. **JSON-RPC standard errors** (`-32700` to `-32603`): Defined by the JSON-RPC 2.0 specification.
/// 2. **JSON-RPC reserved server errors** (`-32099` to `-32000`): Reserved range including
///    `ServerNotInitialized` and `UnknownErrorCode` for backwards compatibility.
/// 3. **LSP reserved errors** (`-32899` to `-32800`): Defined by the LSP specification.
///
/// # Wire Format
///
/// Serialized as a plain integer (e.g., `-32601` for `MethodNotFound`).
///
/// # Variants and Values
///
/// | Variant | Code | Range | Description |
/// |---------|------|-------|-------------|
/// | `ParseError` | -32700 | JSON-RPC | Invalid JSON was received. |
/// | `InvalidRequest` | -32600 | JSON-RPC | The JSON sent is not a valid Request object. |
/// | `MethodNotFound` | -32601 | JSON-RPC | The method does not exist / is not available. |
/// | `InvalidParams` | -32602 | JSON-RPC | Invalid method parameter(s). |
/// | `InternalError` | -32603 | JSON-RPC | Internal JSON-RPC error. |
/// | `ServerNotInitialized` | -32002 | Reserved | Server received a request before `initialize`. |
/// | `UnknownErrorCode` | -32001 | Reserved | Unknown error code. |
/// | `RequestFailed` | -32803 | LSP | Request was syntactically correct but failed. @since 3.17.0. |
/// | `ServerCancelled` | -32802 | LSP | The server cancelled the request. @since 3.17.0. |
/// | `ContentModified` | -32801 | LSP | Content was modified outside normal conditions. |
/// | `RequestCancelled` | -32800 | LSP | The client cancelled the request. |
///
/// # LSP Specification
///
/// See [ErrorCodes](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCodes {
    /// Invalid JSON was received by the server. An error occurred on the server while parsing
    /// the JSON text.
    ParseError,
    /// The JSON sent is not a valid Request object.
    InvalidRequest,
    /// The method does not exist / is not available.
    MethodNotFound,
    /// Invalid method parameter(s).
    InvalidParams,
    /// Internal JSON-RPC error.
    InternalError,

    /// Error code indicating that a server received a notification or request before the
    /// server received the `initialize` request.
    ServerNotInitialized,
    /// Unknown error code.
    UnknownErrorCode,

    /// A request failed but it was syntactically correct, e.g. the method name was known
    /// and the parameters were valid. The error message should contain human readable
    /// information about why the request failed.
    ///
    /// @since 3.17.0
    RequestFailed,
    /// The server cancelled the request. This error code should only be used for requests
    /// that explicitly support being server cancellable.
    ///
    /// @since 3.17.0
    ServerCancelled,
    /// The server detected that the content of a document got modified outside normal
    /// conditions. A server should NOT send this error code if it detects a content change
    /// in its unprocessed messages.
    ContentModified,
    /// The client has canceled a request and a server has detected the cancel.
    RequestCancelled,
}

impl ErrorCodes {
    pub fn code(&self) -> i32 {
        match self {
            ErrorCodes::ParseError => -32700,
            ErrorCodes::InvalidRequest => -32600,
            ErrorCodes::MethodNotFound => -32601,
            ErrorCodes::InvalidParams => -32602,
            ErrorCodes::InternalError => -32603,
            ErrorCodes::ServerNotInitialized => -32002,
            ErrorCodes::UnknownErrorCode => -32001,
            ErrorCodes::RequestFailed => -32803,
            ErrorCodes::ServerCancelled => -32802,
            ErrorCodes::ContentModified => -32801,
            ErrorCodes::RequestCancelled => -32800,
        }
    }

    pub fn from_code(code: i32) -> Option<ErrorCodes> {
        match code {
            -32700 => Some(ErrorCodes::ParseError),
            -32600 => Some(ErrorCodes::InvalidRequest),
            -32601 => Some(ErrorCodes::MethodNotFound),
            -32602 => Some(ErrorCodes::InvalidParams),
            -32603 => Some(ErrorCodes::InternalError),
            -32002 => Some(ErrorCodes::ServerNotInitialized),
            -32001 => Some(ErrorCodes::UnknownErrorCode),
            -32803 => Some(ErrorCodes::RequestFailed),
            -32802 => Some(ErrorCodes::ServerCancelled),
            -32801 => Some(ErrorCodes::ContentModified),
            -32800 => Some(ErrorCodes::RequestCancelled),
            _ => None,
        }
    }
}

impl fmt::Display for ErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl Serialize for ErrorCodes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.code())
    }
}

impl<'de> Deserialize<'de> for ErrorCodes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ErrorCodesVisitor;

        impl<'de> Visitor<'de> for ErrorCodesVisitor {
            type Value = ErrorCodes;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid LSP error code integer")
            }

            fn visit_i64<E>(self, value: i64) -> Result<ErrorCodes, E>
            where
                E: de::Error,
            {
                ErrorCodes::from_code(value as i32).ok_or_else(|| {
                    de::Error::custom(format!("unknown LSP error code: {}", value))
                })
            }

            fn visit_u64<E>(self, value: u64) -> Result<ErrorCodes, E>
            where
                E: de::Error,
            {
                ErrorCodes::from_code(value as i32).ok_or_else(|| {
                    de::Error::custom(format!("unknown LSP error code: {}", value))
                })
            }
        }

        deserializer.deserialize_i32(ErrorCodesVisitor)
    }
}
