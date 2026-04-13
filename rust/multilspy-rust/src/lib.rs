//! Rust Analyzer LSP client library

pub(crate) mod client;
pub(crate) mod config;
pub(crate) mod error;
pub(crate) mod logic;
pub(crate) mod server;

pub use client::LSPClient;
pub use config::RustAnalyzerConfig;
