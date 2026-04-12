//! LSP protocol structure definitions.
//!
//! This module contains Rust implementations of the Language Server Protocol (LSP) 3.17
//! data structures, organized into three sub-modules:
//!
//! - [`common`]: Foundational types shared across requests and responses — `Position`,
//!   `Range`, `Location`, `TextDocumentIdentifier`, `TextDocumentPositionParams`,
//!   `WorkspaceFolder`, `SymbolKind`, `SymbolTag`, `DocumentSymbol`,
//!   `CallHierarchyItem`, `CallHierarchyIncomingCall`, `CallHierarchyOutgoingCall`.
//!
//! - [`requests`]: Request parameter types — `InitializeParams`, `ClientCapabilities`,
//!   and all `*Params` structures for the implemented LSP methods.
//!
//! - [`responses`]: Response result types — `InitializeResult`, `ServerCapabilities`,
//!   `ServerInfo`, capability enums, and response type aliases.
//!
//! # Specification Reference
//!
//! All structures are verified against the
//! [LSP 3.17 Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/).

pub mod common;
pub mod requests;
pub mod responses;
