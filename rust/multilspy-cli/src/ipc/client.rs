use reqwest::Client;
use serde::{Deserialize, Serialize};
use super::super::error::CliError;

#[derive(Serialize)]
pub struct StartRequest {
    pub project_path: String,
}

#[derive(Serialize)]
pub struct PositionRequest {
    pub project_path: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Serialize)]
pub struct DocumentSymbolsRequest {
    pub project_path: String,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct RecursiveRequest {
    pub project_path: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub max_depth: Option<usize>,
}

#[derive(Deserialize)]
pub struct ServerResponse<T> {
    pub status: String,
    pub result: Option<T>,
    pub message: Option<String>,
}

pub struct IpcClient {
    client: Client,
    base_url: String,
    project_path: String,
}

impl IpcClient {
    pub fn new(project_path: String) -> Result<Self, CliError> {
        let mut pid_path = std::env::temp_dir();
        pid_path.push("multilspy-daemon.pid");
        let mut port_path = std::env::temp_dir();
        port_path.push("multilspy-daemon.port");

        let port = std::fs::read_to_string(&port_path)?;
        let port: u16 = port.trim().parse()?;

        Ok(Self {
            client: Client::new(),
            base_url: format!("http://127.0.0.1:{}", port),
            project_path,
        })
    }

    pub async fn health(&self) -> Result<bool, CliError> {
        let response = self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn start(&self) -> Result<(), CliError> {
        let req = StartRequest {
            project_path: self.project_path.clone(),
        };

        let response = self.client
            .post(format!("{}/start", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CliError::Command("Failed to start daemon".to_string()));
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), CliError> {
        let response = self.client
            .post(format!("{}/stop", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CliError::Command("Failed to stop daemon".to_string()));
        }

        Ok(())
    }

    async fn request_position<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<T, CliError> {
        let req = PositionRequest {
            project_path: self.project_path.clone(),
            file_path,
            line,
            column,
        };

        let response = self.client
            .post(format!("{}{}", self.base_url, endpoint))
            .json(&req)
            .send()
            .await?;

        let status = response.status();
        let result: ServerResponse<T> = response.json().await?;

        if !status.is_success() || result.status == "error" {
            return Err(CliError::Command(result.message.unwrap_or_else(|| "Unknown error".to_string())));
        }

        result.result.ok_or_else(|| CliError::Command("Missing result".to_string()))
    }

    pub async fn definition(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<serde_json::Value, CliError> {
        self.request_position("/definition", file_path, line, column).await
    }

    pub async fn type_definition(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<serde_json::Value, CliError> {
        self.request_position("/type_definition", file_path, line, column).await
    }

    pub async fn references(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<serde_json::Value, CliError> {
        self.request_position("/references", file_path, line, column).await
    }

    pub async fn document_symbols(
        &self,
        file_path: String,
    ) -> Result<serde_json::Value, CliError> {
        let req = DocumentSymbolsRequest {
            project_path: self.project_path.clone(),
            file_path,
        };

        let response = self.client
            .post(format!("{}/document_symbols", self.base_url))
            .json(&req)
            .send()
            .await?;

        let status = response.status();
        let result: ServerResponse<serde_json::Value> = response.json().await?;

        if !status.is_success() || result.status == "error" {
            return Err(CliError::Command(result.message.unwrap_or_else(|| "Unknown error".to_string())));
        }

        result.result.ok_or_else(|| CliError::Command("Missing result".to_string()))
    }

    pub async fn implementation(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<serde_json::Value, CliError> {
        self.request_position("/implementation", file_path, line, column).await
    }

    pub async fn incoming_calls(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<serde_json::Value, CliError> {
        self.request_position("/incoming_calls", file_path, line, column).await
    }

    pub async fn outgoing_calls(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<serde_json::Value, CliError> {
        self.request_position("/outgoing_calls", file_path, line, column).await
    }

    async fn request_recursive<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        file_path: String,
        line: u32,
        column: u32,
        max_depth: Option<usize>,
    ) -> Result<T, CliError> {
        let req = RecursiveRequest {
            project_path: self.project_path.clone(),
            file_path,
            line,
            column,
            max_depth,
        };

        let response = self.client
            .post(format!("{}{}", self.base_url, endpoint))
            .json(&req)
            .send()
            .await?;

        let status = response.status();
        let result: ServerResponse<T> = response.json().await?;

        if !status.is_success() || result.status == "error" {
            return Err(CliError::Command(result.message.unwrap_or_else(|| "Unknown error".to_string())));
        }

        result.result.ok_or_else(|| CliError::Command("Missing result".to_string()))
    }

    pub async fn incoming_calls_recursive(
        &self,
        file_path: String,
        line: u32,
        column: u32,
        max_depth: Option<usize>,
    ) -> Result<serde_json::Value, CliError> {
        self.request_recursive("/incoming_calls_recursive", file_path, line, column, max_depth).await
    }

    pub async fn outgoing_calls_recursive(
        &self,
        file_path: String,
        line: u32,
        column: u32,
        max_depth: Option<usize>,
    ) -> Result<serde_json::Value, CliError> {
        self.request_recursive("/outgoing_calls_recursive", file_path, line, column, max_depth).await
    }
}
