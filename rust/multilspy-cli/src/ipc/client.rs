#![allow(dead_code)]

use reqwest::Client;
use serde::{Deserialize, Serialize};
use super::super::error::CliError;

#[derive(Serialize)]
pub struct StartRequest {
    pub project_root: String,
}

#[derive(Serialize)]
pub struct PositionRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Serialize)]
pub struct ReferencesRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub include_declaration: bool,
}

#[derive(Serialize)]
pub struct RecursiveRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub max_depth: Option<usize>,
}

#[derive(Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
}

pub struct IpcClient {
    client: Client,
    base_url: String,
}

impl IpcClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://127.0.0.1:3000".to_string(),
        }
    }

    pub async fn health(&self) -> Result<bool, CliError> {
        let response = self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn start(&self, project_root: String) -> Result<(), CliError> {
        let req = StartRequest { project_root };

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
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}
