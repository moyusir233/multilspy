use std::process::{Command, Stdio};
use super::pid_file::PidFile;
use super::port_file::PortFile;
use super::super::error::CliError;

pub struct DaemonManager {
    pid_file: PidFile,
    port_file: PortFile,
}

impl DaemonManager {
    pub fn new() -> Self {
        let mut pid_path = std::env::temp_dir();
        pid_path.push("multilspy-daemon.pid");
        let mut port_path = std::env::temp_dir();
        port_path.push("multilspy-daemon.port");

        Self {
            pid_file: PidFile::new(pid_path),
            port_file: PortFile::new(port_path),
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

        // Get the path to the current executable to re-run ourselves
        let current_exe = std::env::current_exe()?;

        // Spawn the daemon process
        let _child = Command::new(current_exe)
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Wait a little for the daemon to write the PID and port files
        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }

    pub fn stop(&self) -> Result<(), CliError> {
        if !self.is_running() {
            return Err(CliError::DaemonNotRunning);
        }

        let pid = self.pid_file.read()?;

        #[cfg(target_os = "windows")]
        {
            Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Try SIGTERM first, then SIGKILL
            let _ = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Check if process is still running
            if unsafe { libc::kill(pid as i32, 0) } == 0 {
                unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            }
        }

        self.pid_file.remove()?;
        self.port_file.remove()?;

        Ok(())
    }

    pub fn restart(&self) -> Result<(), CliError> {
        let _ = self.stop();
        self.start()
    }

    pub fn status(&self) -> Result<Option<(u32, u16)>, CliError> {
        if self.is_running() {
            let pid = self.pid_file.read()?;
            let port = self.port_file.read()?;
            Ok(Some((pid, port)))
        } else {
            Ok(None)
        }
    }
}

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn run_daemon() -> Result<(), CliError> {
    let mut pid_path = std::env::temp_dir();
    pid_path.push("multilspy-daemon.pid");
    let mut port_path = std::env::temp_dir();
    port_path.push("multilspy-daemon.port");

    let pid_file = PidFile::new(pid_path);
    let port_file = PortFile::new(port_path);

    // Write PID file
    let pid = std::process::id();
    pid_file.write(pid)?;

    // Start server and get port
    let port = crate::ipc::server::start_server().await?;

    // Write port file
    port_file.write(port)?;

    // Wait forever
    tokio::signal::ctrl_c().await?;

    // Cleanup
    let _ = pid_file.remove();
    let _ = port_file.remove();

    Ok(())
}
