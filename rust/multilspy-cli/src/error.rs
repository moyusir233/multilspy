use thiserror::Error;
use multilspy_rust::error::ServerError;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Server error: {0}")]
    Server(#[from] ServerError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Daemon already running")]
    DaemonAlreadyRunning,

    #[error("Command error: {0}")]
    Command(String),
}
