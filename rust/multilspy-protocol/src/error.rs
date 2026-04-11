use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Request ID mismatch")]
    RequestIdMismatch,

    #[error("Transport closed")]
    TransportClosed,
}
