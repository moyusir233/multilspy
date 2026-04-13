use std::fs::File;
use std::path::{Path, PathBuf};

use crate::ipc;

#[derive(Debug)]
pub struct DaemonInfo {
    pub pid: u32,
    pub port: u16,
}

fn workspace_hash(workspace: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    workspace.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn pidfile_dir() -> PathBuf {
    std::env::temp_dir().join("multilspy-cli")
}

fn pidfile_path(workspace: &Path) -> PathBuf {
    pidfile_dir().join(format!("{}.json", workspace_hash(workspace)))
}

fn lockfile_path(workspace: &Path) -> PathBuf {
    pidfile_dir().join(format!("{}.lock", workspace_hash(workspace)))
}

pub fn write_pidfile(workspace: &Path, pid: u32, port: u16) -> anyhow::Result<()> {
    let dir = pidfile_dir();
    std::fs::create_dir_all(&dir)?;
    let path = pidfile_path(workspace);
    let data = serde_json::json!({
        "pid": pid,
        "port": port,
        "workspace": workspace.display().to_string(),
    });
    std::fs::write(&path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

pub fn read_pidfile(workspace: &Path) -> Option<DaemonInfo> {
    let path = pidfile_path(workspace);
    let content = std::fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    let pid = data.get("pid")?.as_u64()? as u32;
    let port = data.get("port")?.as_u64()? as u16;
    Some(DaemonInfo { pid, port })
}

pub fn remove_pidfile(workspace: &Path) -> anyhow::Result<()> {
    let path = pidfile_path(workspace);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

fn acquire_lock(workspace: &Path) -> anyhow::Result<File> {
    std::fs::create_dir_all(pidfile_dir())?;
    let lock_path = lockfile_path(workspace);
    let file = File::options()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)?;

    use std::os::unix::io::AsRawFd;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if rc != 0 {
        anyhow::bail!(
            "failed to acquire lock on {:?}: {}",
            lock_path,
            std::io::Error::last_os_error()
        );
    }
    Ok(file)
}

fn release_lock(file: &File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
}

pub async fn ensure_daemon(workspace: &Path, initialize_params_path: &Path) -> anyhow::Result<u16> {
    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());

    let lock = acquire_lock(&canonical)?;

    let result = ensure_daemon_inner(&canonical, initialize_params_path).await;

    release_lock(&lock);
    drop(lock);

    result
}

async fn ensure_daemon_inner(
    canonical: &Path,
    initialize_params_path: &Path,
) -> anyhow::Result<u16> {
    if let Some(info) = read_pidfile(canonical) {
        if is_process_alive(info.pid) && ipc::ping(info.port).await {
            tracing::debug!(
                "daemon already running: pid={}, port={}",
                info.pid,
                info.port
            );
            return Ok(info.port);
        }
        tracing::info!("stale daemon detected, cleaning up pidfile");
        let _ = remove_pidfile(canonical);
    }

    spawn_daemon(canonical, initialize_params_path).await
}

async fn spawn_daemon(workspace: &Path, initialize_params_path: &Path) -> anyhow::Result<u16> {
    let exe = std::env::current_exe()?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("daemon")
        .arg("--workspace")
        .arg(workspace)
        .arg("--initialize-params")
        .arg(initialize_params_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    cmd.spawn()?;

    for attempt in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        if let Some(info) = read_pidfile(workspace)
            && ipc::ping(info.port).await
        {
            tracing::info!(
                "daemon started: pid={}, port={} (attempt {})",
                info.pid,
                info.port,
                attempt
            );
            return Ok(info.port);
        }
    }

    anyhow::bail!("failed to start daemon within 30 seconds")
}
