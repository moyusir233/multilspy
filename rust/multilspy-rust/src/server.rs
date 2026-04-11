use std::process::{Command, Stdio};
use tokio::process::Child;
use multilspy_protocol::transport::Transport;
use multilspy_protocol::json_rpc::{Request, RequestId, Notification};
use multilspy_protocol::protocol::requests::{InitializeParams, ClientCapabilities};
use super::config::RustAnalyzerConfig;
use super::error::ServerError;

pub type StdioTransport = Transport<tokio::process::ChildStdout, tokio::process::ChildStdin>;

#[derive(Debug)]
pub struct RustAnalyzerServer {
    config: RustAnalyzerConfig,
    child: Option<Child>,
    transport: Option<StdioTransport>,
    next_request_id: u64,
}

impl RustAnalyzerServer {
    pub fn new(config: RustAnalyzerConfig) -> Self {
        Self {
            config,
            child: None,
            transport: None,
            next_request_id: 1,
        }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        if self.child.is_some() {
            return Err(ServerError::ServerAlreadyRunning);
        }

        // Spawn rust-analyzer process
        let mut cmd = Command::new(&self.config.server_path);

        // Set working directory to project root
        cmd.current_dir(&self.config.project_root);

        // Add environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        cmd.stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = tokio::process::Command::from(cmd)
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| ServerError::IoError(std::io::Error::other("Failed to get stdout")))?;
        let stdin = child.stdin.take().ok_or_else(|| ServerError::IoError(std::io::Error::other("Failed to get stdin")))?;

        let transport = Transport::new(stdout, stdin);

        self.child = Some(child);
        self.transport = Some(transport);

        // Initialize server
        self.initialize().await?;

        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), ServerError> {
        // Get next request ID first to avoid borrow conflict
        let request_id = self.next_request_id();

        let transport = self.transport.as_mut().ok_or(ServerError::ServerNotRunning)?;

        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(format!("file://{}", self.config.project_root.to_string_lossy())),
            capabilities: ClientCapabilities::default(),
            trace: Some("off".to_string()),
            workspace_folders: Some(vec![]),
        };

        let request = Request::new(
            request_id,
            "initialize".to_string(),
            Some(serde_json::to_value(&params)?),
        );

        transport.send_request(&request).await?;

        let response = transport.receive_response().await?;

        match response.result {
            multilspy_protocol::json_rpc::ResponseResult::Result(_) => {
                // Send initialized notification
                let notification = Notification {
                    jsonrpc: "2.0".to_string(),
                    method: "initialized".to_string(),
                    params: Some(serde_json::json!({})),
                };

                transport.send_notification(&notification).await?;

                Ok(())
            }
            multilspy_protocol::json_rpc::ResponseResult::Error(err) => {
                Err(ServerError::InitializationFailed(format!("{} (code: {})", err.message, err.code)))
            }
        }
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        if self.child.is_none() {
            return Err(ServerError::ServerNotRunning);
        }

        // Get next request ID first to avoid borrow conflict
        let request_id = self.next_request_id();

        // Send shutdown request
        let transport = self.transport.as_mut().ok_or(ServerError::ServerNotRunning)?;

        let request = Request::new(
            request_id,
            "shutdown".to_string(),
            None,
        );

        transport.send_request(&request).await?;
        let _response = transport.receive_response().await?;

        // Send exit notification
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "exit".to_string(),
            params: None,
        };

        transport.send_notification(&notification).await?;

        // Wait for process to exit
        if let Some(mut child) = self.child.take() {
            let status = child.wait().await?;
            if !status.success() {
                tracing::warn!("Server exited with non-zero status: {}", status);
            }
        }

        self.transport = None;

        Ok(())
    }

    pub async fn send_request<T: serde::Serialize>(&mut self, method: String, params: Option<T>) -> Result<serde_json::Value, ServerError> {
        // Get next request ID first to avoid borrow conflict
        let request_id = self.next_request_id();

        let transport = self.transport.as_mut().ok_or(ServerError::ServerNotRunning)?;

        let request = Request::new(
            request_id.clone(),
            method,
            params.map(|p| serde_json::to_value(p)).transpose()?,
        );

        transport.send_request(&request).await?;
        let response = transport.receive_response().await?;

        if response.id != request_id {
            return Err(ServerError::ProtocolError(multilspy_protocol::error::ProtocolError::RequestIdMismatch));
        }

        match response.result {
            multilspy_protocol::json_rpc::ResponseResult::Result(result) => Ok(result),
            multilspy_protocol::json_rpc::ResponseResult::Error(err) => {
                Err(ServerError::ProtocolError(multilspy_protocol::error::ProtocolError::InvalidMessage(format!("Request failed: {} (code: {})", err.message, err.code))))
            }
        }
    }

    fn next_request_id(&mut self) -> RequestId {
        let id = RequestId::Number(self.next_request_id);
        self.next_request_id += 1;
        id
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }
}

impl Drop for RustAnalyzerServer {
    fn drop(&mut self) {
        if self.is_running() {
            // Best effort to stop the server when dropped
            let _ = tokio::runtime::Runtime::new().unwrap().block_on(self.stop());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_server_lifecycle() {
        // Skip test if rust-analyzer is not installed
        if Command::new("rust-analyzer").output().is_err() {
            println!("rust-analyzer not installed, skipping test");
            return;
        }

        let config = RustAnalyzerConfig::new(Path::new(".").to_path_buf());
        let mut server = RustAnalyzerServer::new(config);

        assert!(!server.is_running());

        // Start server
        let start_result = server.start().await;
        assert!(start_result.is_ok());
        assert!(server.is_running());

        // Stop server
        let stop_result = server.stop().await;
        assert!(stop_result.is_ok());
        assert!(!server.is_running());
    }
}
