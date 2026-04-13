use multilspy_protocol::error::ProtocolError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Protocol error: {0}")]
    ProtocolError(#[from] ProtocolError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Server already running")]
    ServerAlreadyRunning,

    #[error("Server not running")]
    ServerNotRunning,

    #[error("Server initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Server exited with code: {0}")]
    ServerExited(i32),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Other internal error: {0}")]
    Others(#[from] anyhow::Error),
}
