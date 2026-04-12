
use multilspy_rust::client::LspClient;
use multilspy_rust::config::RustAnalyzerConfig;
use std::path::PathBuf;

#[test]
fn test_lsp_client_creation() {
    let config = RustAnalyzerConfig::new(PathBuf::from("."));
    let _client = LspClient::new(config);
}

