use crate::commands::utils::{resolve_project_path, resolve_file_path, ensure_daemon_running};
use crate::error::CliError;
use serde_json::json;

pub async fn handle(project: Option<&str>, file: &str, line: u32, column: u32) -> Result<(), CliError> {
    let project_path = resolve_project_path(project)?;
    let file_path = resolve_file_path(&project_path, file);
    let client = ensure_daemon_running(&project_path).await?;

    let result = client.definition(file_path, line, column).await?;
    println!("{}", json!({"status": "ok", "result": result}));

    Ok(())
}
