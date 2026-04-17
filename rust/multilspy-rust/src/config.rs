use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustAnalyzerConfig {
    /// Path to rust-analyzer executable
    pub server_executable_path: PathBuf,
    /// Root directory of the Rust project
    pub project_root: PathBuf,
    /// Additional environment variables to pass to rust-analyzer
    pub env: Vec<(String, String)>,
    /// Rust analyzer stderr log file path
    /// If not set, the log will be printed to the console.
    pub ra_stderr_log_path: Option<PathBuf>,
    /// InitializeParams json file path
    pub initialize_params_path: PathBuf,
    /// 等待lsp server发起workdoneProgress创建请求的最大时间窗口，默认60秒
    pub wait_work_done_progress_create_max_time: Duration,
    /// 是否需要保证任意关于文档的lsp请求操作都需要执行`textDocument/didOpen`与`textDocument/didClose` notification
    /// 如果是纯静态的分析，即磁盘上的文件内容总是可靠的，则不需要打开这个选项
    pub need_open_file: bool,
}

fn get_rust_analyzer_path() -> anyhow::Result<PathBuf> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg("which rust-analyzer")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get rust-analyzer path: {:?}", e))?;
    let path = String::from_utf8_lossy(&output.stdout);
    let path_buf = PathBuf::from(path.as_ref().trim());
    Ok(path_buf)
}

impl RustAnalyzerConfig {
    pub fn new(project_root: PathBuf, initialize_params_path: PathBuf) -> Self {
        let mut config = Self {
            project_root: project_root.canonicalize().unwrap_or(project_root),
            initialize_params_path: initialize_params_path
                .canonicalize()
                .unwrap_or(initialize_params_path),
            // 默认使用`bash -c "which rust-analyzer"`来获得rust-analyzer的路径
            server_executable_path: get_rust_analyzer_path().unwrap_or_default(),
            env: vec![("RA_LOG".to_string(), "info".to_string())],
            ra_stderr_log_path: None,
            wait_work_done_progress_create_max_time: Duration::from_secs(30),
            need_open_file: false,
        };

        if let Ok(current_dir) = std::env::current_dir() {
            if config.project_root.starts_with(".") {
                config.project_root = current_dir.join(&config.project_root);
            }
            if config.initialize_params_path.starts_with(".") {
                config.initialize_params_path = current_dir.join(&config.initialize_params_path);
            }
        }

        config.ra_stderr_log_path = Some(config.project_root.join("multilspy-ra.log"));

        config
    }

    pub fn with_server_path(mut self, server_path: PathBuf) -> Self {
        self.server_executable_path = server_path;
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.push((key, value));
        self
    }

    pub fn with_stderr_log_path(mut self, path: PathBuf) -> Self {
        self.ra_stderr_log_path = Some(path);
        self
    }

    pub fn with_wait_work_done_progress_create_max_time(mut self, duration: Duration) -> Self {
        self.wait_work_done_progress_create_max_time = duration;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_config_builder() {
        let project_root = Path::new("/test/project");
        let mut config = RustAnalyzerConfig::new(
            project_root.to_path_buf(),
            PathBuf::from("ra_initialize_params.json"),
        )
        .with_server_path(Path::new("/usr/bin/rust-analyzer").to_path_buf())
        .with_env("RUST_LOG".to_string(), "info".to_string());

        assert_eq!(config.project_root, project_root);
        assert_eq!(
            config.server_executable_path,
            Path::new("/usr/bin/rust-analyzer")
        );

        let mut expected_env = vec![
            ("RUST_LOG".to_string(), "info".to_string()),
            ("RA_LOG".to_string(), "info".to_string()),
        ];
        assert_eq!(config.env.len(), expected_env.len());

        expected_env.sort();
        config.env.sort();

        assert_eq!(config.env, expected_env);
    }
}
