use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustAnalyzerConfig {
    /// Path to rust-analyzer executable
    pub server_path: PathBuf,

    /// Root directory of the Rust project
    pub project_root: PathBuf,

    /// Additional environment variables to pass to rust-analyzer
    pub env: Vec<(String, String)>,

    /// Rust analyzer configuration settings
    pub settings: serde_json::Value,
}

impl Default for RustAnalyzerConfig {
    fn default() -> Self {
        Self {
            server_path: PathBuf::from("rust-analyzer"),
            project_root: std::env::current_dir().unwrap_or_default(),
            env: Vec::new(),
            settings: serde_json::json!({
                "rust-analyzer": {
                    "diagnostics": {
                        "enable": true
                    },
                    "procMacro": {
                        "enable": true
                    },
                    "cargo": {
                        "loadOutDirsFromCheck": true
                    }
                }
            }),
        }
    }
}

impl RustAnalyzerConfig {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            ..Default::default()
        }
    }

    pub fn with_server_path(mut self, server_path: PathBuf) -> Self {
        self.server_path = server_path;
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.push((key, value));
        self
    }

    pub fn with_setting(mut self, key: String, value: serde_json::Value) -> Self {
        if let Some(serde_json::Value::Object(map)) = self.settings.get_mut("rust-analyzer") {
            map.insert(key, value);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_config_default() {
        let config = RustAnalyzerConfig::default();
        assert_eq!(config.server_path, Path::new("rust-analyzer"));
        assert!(config.settings.get("rust-analyzer").is_some());
    }

    #[test]
    fn test_config_builder() {
        let project_root = Path::new("/test/project");
        let config = RustAnalyzerConfig::new(project_root.to_path_buf())
            .with_server_path(Path::new("/usr/bin/rust-analyzer").to_path_buf())
            .with_env("RUST_LOG".to_string(), "info".to_string())
            .with_setting("diagnostics.enable".to_string(), serde_json::json!(false));

        assert_eq!(config.project_root, project_root);
        assert_eq!(config.server_path, Path::new("/usr/bin/rust-analyzer"));
        assert_eq!(config.env.len(), 1);
        assert_eq!(config.env[0], ("RUST_LOG".to_string(), "info".to_string()));
    }
}
