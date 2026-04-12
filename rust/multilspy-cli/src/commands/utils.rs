use std::path::{Path, PathBuf};
use crate::error::CliError;
use crate::daemon::manager::DaemonManager;
use crate::ipc::client::IpcClient;

pub fn resolve_project_path(project_path: Option<&str>) -> Result<String, CliError> {
    let path = match project_path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir()?,
    };

    let mut current = if path.is_file() {
        path.parent().ok_or_else(|| CliError::Command("Invalid path".to_string()))?.to_path_buf()
    } else {
        path
    };

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(current.to_string_lossy().to_string());
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    Ok(current.to_string_lossy().to_string())
}

pub fn resolve_file_path(project_path: &str, file_path: &str) -> String {
    let project_path = Path::new(project_path);
    let file_path = Path::new(file_path);

    if file_path.is_absolute() {
        if let Ok(relative) = file_path.strip_prefix(project_path) {
            return relative.to_string_lossy().to_string();
        }
    }

    file_path.to_string_lossy().to_string()
}

pub async fn ensure_daemon_running(project_path: &str) -> Result<IpcClient, CliError> {
    let daemon_manager = DaemonManager::new();

    if !daemon_manager.is_running() {
        daemon_manager.start()?;

        // Wait a bit for daemon to start
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    let client = IpcClient::new(project_path.to_string())?;

    // Start the LSP instance for the project
    client.start().await?;

    Ok(client)
}
