use std::process::Command;
use super::pid_file::PidFile;
use super::super::error::CliError;

pub struct DaemonManager {
    pid_file: PidFile,
}

impl DaemonManager {
    pub fn new() -> Self {
        let mut path = std::env::temp_dir();
        path.push("multilspy-daemon.pid");

        Self {
            pid_file: PidFile::new(path),
        }
    }

    pub fn is_running(&self) -> bool {
        if !self.pid_file.exists() {
            return false;
        }

        if let Ok(pid) = self.pid_file.read() {
            // Check if process is still running (platform-specific)
            #[cfg(target_os = "windows")]
            {
                let output = Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid)])
                    .output();
                output.map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string())).unwrap_or(false)
            }
            #[cfg(not(target_os = "windows"))]
            {
                let output = Command::new("ps")
                    .args(["-p", &pid.to_string()])
                    .output();
                output.map(|o| o.status.success()).unwrap_or(false)
            }
        } else {
            false
        }
    }

    pub fn start(&self) -> Result<(), CliError> {
        if self.is_running() {
            return Err(CliError::DaemonAlreadyRunning);
        }

        // Start daemon (placeholder - full implementation in later tasks)
        println!("Starting daemon...");

        Ok(())
    }

    pub fn stop(&self) -> Result<(), CliError> {
        if !self.is_running() {
            return Err(CliError::DaemonNotRunning);
        }

        let pid = self.pid_file.read()?;

        // Stop daemon (placeholder - full implementation in later tasks)
        println!("Stopping daemon with PID {}...", pid);
        self.pid_file.remove()?;

        Ok(())
    }

    pub fn restart(&self) -> Result<(), CliError> {
        let _ = self.stop();
        self.start()
    }

    pub fn status(&self) -> Result<(), CliError> {
        if self.is_running() {
            let pid = self.pid_file.read()?;
            println!("Daemon is running with PID {}", pid);
        } else {
            println!("Daemon is not running");
        }
        Ok(())
    }
}

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new()
    }
}
