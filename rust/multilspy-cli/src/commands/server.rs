use clap::Subcommand;
use crate::daemon::manager::DaemonManager;
use crate::error::CliError;
use serde_json::json;

#[derive(Subcommand, Debug)]
pub enum ServerCommand {
    /// Start the multilspy daemon
    Start,
    /// Stop the multilspy daemon
    Stop,
    /// Restart the multilspy daemon
    Restart,
    /// Check the status of the multilspy daemon
    Status,
}

pub async fn handle(command: &ServerCommand) -> Result<(), CliError> {
    let daemon_manager = DaemonManager::new();

    match command {
        ServerCommand::Start => {
            if daemon_manager.is_running() {
                println!("{}", json!({"status": "already_running"}));
            } else {
                daemon_manager.start()?;
                println!("{}", json!({"status": "ok"}));
            }
        }
        ServerCommand::Stop => {
            if daemon_manager.is_running() {
                daemon_manager.stop()?;
                println!("{}", json!({"status": "ok"}));
            } else {
                println!("{}", json!({"status": "not_running"}));
            }
        }
        ServerCommand::Restart => {
            let _ = daemon_manager.stop();
            daemon_manager.start()?;
            println!("{}", json!({"status": "ok"}));
        }
        ServerCommand::Status => {
            match daemon_manager.status()? {
                Some((_pid, port)) => {
                    println!("{}", json!({
                        "status": "running",
                        "address": format!("127.0.0.1:{}", port)
                    }));
                }
                None => {
                    println!("{}", json!({
                        "status": "stopped",
                        "address": null
                    }));
                }
            }
        }
    }

    Ok(())
}
