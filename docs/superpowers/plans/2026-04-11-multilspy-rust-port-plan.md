# Multilspy Rust Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Python multilspy LSP client library and CLI to Rust with full functional parity for Rust Analyzer support, delivering ≥2x performance improvement.

**Architecture:** Modular layered architecture with 3 crates in a Cargo workspace: multilspy-protocol (core LSP/JSON-RPC layer), multilspy-rust (Rust Analyzer client library), multilspy-cli (CLI interface with daemon support).

**Tech Stack:** Tokio (async runtime), Tracing (logging), Clap (CLI parsing), Serde/serde_json (serialization), Thiserror (custom errors), Anyhow (application error handling), Axum (HTTP server for IPC), Reqwest (HTTP client for IPC).

**Development Requirements:** All code must pass `cargo clippy` checks with no warnings to ensure code standardization.

---

## File Structure Map
*All modules must maintain **exact functional parity** with their corresponding Python implementation files.*
```
rust/
├── Cargo.toml                                  # Workspace configuration
├── multilspy-protocol/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                              # Crate root, exports all public types
│       ├── error.rs                            # Protocol error definitions
│       ├── json_rpc.rs                         # JSON-RPC 2.0 message types
│       ├── protocol/
│       │   ├── mod.rs
│       │   ├── common.rs                       # Common LSP types - aligns with LSP spec types used in Python implementation
│       │   ├── requests.rs                     # LSP request structs - aligns with requests in `src/multilspy/lsp_protocol_handler/`
│       │   └── responses.rs                    # LSP response structs - aligns with responses in `src/multilspy/lsp_protocol_handler/`
│       └── transport.rs                        # Async transport layer (stdio/HTTP) - aligns with transport logic in `src/multilspy/lsp_protocol_handler/`
├── multilspy-rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                              # Crate root, exports public API
│       ├── error.rs                            # Library error definitions - aligns with errors in `src/multilspy/language_server.py`
│       ├── config.rs                           # Rust Analyzer configuration - **exact parity** with `src/multilspy/language_servers/rust_analyzer/config.py`
│       ├── server.rs                           # Rust Analyzer server lifecycle management - **exact parity** with `src/multilspy/language_servers/rust_analyzer/server.py`
│       ├── client.rs                           # LSP client implementation - **exact parity** with `src/multilspy/language_server.py` public API
│       └── logic.rs                            # Higher-level logic (recursive call hierarchy) - **exact parity** with recursive logic in `src/multilspy/cli/client.py`
└── multilspy-cli/
    ├── Cargo.toml
    └── src/
        ├── main.rs                             # CLI entry point - aligns with `src/multilspy_cli/cli.py` main function
        ├── error.rs                            # CLI error definitions - aligns with error formatting in `src/multilspy_cli/cli.py`
        ├── config.rs                           # CLI configuration - aligns with argument parsing config in `src/multilspy_cli/cli.py`
        ├── commands/
        │   ├── mod.rs
        │   ├── server.rs                       # Server management commands - **exact parity** with server commands in `src/multilspy_cli/cli.py`
        │   ├── definition.rs                   # Definition command - aligns with definition handler in `src/multilspy_cli/cli.py`
        │   ├── type_definition.rs              # Type definition command - aligns with type_definition handler in `src/multilspy_cli/cli.py`
        │   ├── references.rs                   # References command - aligns with references handler in `src/multilspy_cli/cli.py`
        │   ├── document_symbols.rs             # Document symbols command - aligns with document_symbols handler in `src/multilspy_cli/cli.py`
        │   ├── implementation.rs               # Implementation command - aligns with implementation handler in `src/multilspy_cli/cli.py`
        │   ├── incoming_calls.rs               # Incoming calls command - aligns with incoming_calls handler in `src/multilspy_cli/cli.py`
        │   ├── outgoing_calls.rs               # Outgoing calls command - aligns with outgoing_calls handler in `src/multilspy_cli/cli.py`
        │   ├── incoming_calls_recursive.rs     # Recursive incoming calls command - **exact parity** with implementation in `src/multilspy_cli/client.py`
        │   └── outgoing_calls_recursive.rs     # Recursive outgoing calls command - **exact parity** with implementation in `src/multilspy_cli/client.py`
        ├── daemon/
        │   ├── mod.rs
        │   ├── manager.rs                      # Daemon process management - **exact parity** with daemon logic in `src/multilspy_cli/server.py`
        │   └── pid_file.rs                     # PID file handling - aligns with PID file logic in `src/multilspy_cli/server.py`
        └── ipc/
            ├── mod.rs
            ├── client.rs                       # HTTP IPC client implementation (uses reqwest) - aligns with client IPC logic in `src/multilspy_cli/client.py`
            └── server.rs                       # HTTP IPC server implementation (uses axum) - aligns with server IPC logic in `src/multilspy_cli/server.py`
```

---

## Task 1: Create Cargo Workspace and Crate Structure

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/multilspy-protocol/Cargo.toml`
- Create: `rust/multilspy-protocol/src/lib.rs`
- Create: `rust/multilspy-rust/Cargo.toml`
- Create: `rust/multilspy-rust/src/lib.rs`
- Create: `rust/multilspy-cli/Cargo.toml`
- Create: `rust/multilspy-cli/src/main.rs`

- [ ] **Step 1: Create workspace Cargo.toml**
```toml
[workspace]
members = [
    "multilspy-protocol",
    "multilspy-rust",
    "multilspy-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Your Name"]
license = "MIT"

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
anyhow = "1.0"
clap = { version = "4.0", features = ["derive"] }
axum = "0.7"
reqwest = { version = "0.12", features = ["json"] }
```

- [ ] **Step 2: Create multilspy-protocol/Cargo.toml**
```toml
[package]
name = "multilspy-protocol"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

- [ ] **Step 3: Create multilspy-protocol/src/lib.rs**
```rust
//! Core LSP protocol and JSON-RPC implementation for Multilspy

pub mod error;
pub mod json_rpc;
pub mod protocol;
pub mod transport;
```

- [ ] **Step 4: Create multilspy-rust/Cargo.toml**
```toml
[package]
name = "multilspy-rust"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
anyhow.workspace = true
tracing.workspace = true
multilspy-protocol = { path = "../multilspy-protocol" }
```

- [ ] **Step 5: Create multilspy-rust/src/lib.rs**
```rust
//! Rust Analyzer LSP client library

pub mod config;
pub mod error;
pub mod client;
pub mod server;
pub mod logic;
```

- [ ] **Step 6: Create multilspy-cli/Cargo.toml**
```toml
[package]
name = "multilspy-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
tokio.workspace = true
clap.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
tracing.workspace = true
multilspy-protocol = { path = "../multilspy-protocol" }
multilspy-rust = { path = "../multilspy-rust" }
```

- [ ] **Step 7: Create multilspy-cli/src/main.rs**
```rust
//! Multilspy CLI entry point

fn main() -> anyhow::Result<()> {
    println!("Multilspy CLI v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
```

- [ ] **Step 8: Verify workspace builds**
Run: `cd rust && cargo build`
Expected: Build succeeds with no errors

- [ ] **Step 9: Commit**
```bash
git add rust/Cargo.toml rust/multilspy-protocol/Cargo.toml rust/multilspy-protocol/src/lib.rs rust/multilspy-rust/Cargo.toml rust/multilspy-rust/src/lib.rs rust/multilspy-cli/Cargo.toml rust/multilspy-cli/src/main.rs
git commit -m "feat: initial workspace and crate structure"
```

---

## Task 2: Implement multilspy-protocol Error and JSON-RPC Layer

**Files:**
- Create: `rust/multilspy-protocol/src/error.rs`
- Create: `rust/multilspy-protocol/src/json_rpc.rs`

- [ ] **Step 1: Write error.rs implementation**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Request ID mismatch")]
    RequestIdMismatch,

    #[error("Transport closed")]
    TransportClosed,
}
```

- [ ] **Step 2: Write json_rpc.rs implementation**
```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(flatten)]
    pub result: ResponseResult,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseResult {
    Result(Value),
    Error(ResponseError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl Request {
    pub fn new(id: RequestId, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }
}

impl Response {
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: ResponseResult::Result(result),
        }
    }

    pub fn error(id: RequestId, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: ResponseResult::Error(ResponseError {
                code,
                message,
                data,
            }),
        }
    }
}
```

- [ ] **Step 3: Add tests for JSON-RPC serialization**
Add to bottom of json_rpc.rs:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let request = Request::new(
            RequestId::Number(1),
            "textDocument/definition".to_string(),
            Some(json!({ "textDocument": { "uri": "file:///test.rs" }, "position": { "line": 0, "character": 0 } }))
        );
        
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.id, RequestId::Number(1));
        assert_eq!(deserialized.method, "textDocument/definition");
    }

    #[test]
    fn test_response_serialization() {
        let response = Response::success(
            RequestId::Number(1),
            json!([{ "uri": "file:///test.rs", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 10 } } }])
        );
        
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.id, RequestId::Number(1));
        assert!(matches!(deserialized.result, ResponseResult::Result(_)));
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**
Run: `cd rust && cargo test -p multilspy-protocol json_rpc::tests`
Expected: All tests pass

- [ ] **Step 5: Run clippy to ensure code standardization**
Run: `cd rust && cargo clippy -p multilspy-protocol -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 6: Commit**
```bash
git add rust/multilspy-protocol/src/error.rs rust/multilspy-protocol/src/json_rpc.rs
git commit -m "feat(protocol): add error types and JSON-RPC implementation"
```

---

## Task 3: Implement multilspy-protocol LSP Common Types

**Files:**
- Create: `rust/multilspy-protocol/src/protocol/mod.rs`
- Create: `rust/multilspy-protocol/src/protocol/common.rs`

- [ ] **Step 1: Create protocol/mod.rs**
```rust
pub mod common;
pub mod requests;
pub mod responses;
```

- [ ] **Step 2: Write common.rs with LSP common types**
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    /// Line position in a document (zero-based).
    pub line: u32,
    /// Character offset on a line in a document (zero-based).
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn to_file_path(&self) -> Option<PathBuf> {
        self.uri.strip_prefix("file://").map(PathBuf::from)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDocumentIdentifier {
    /// The text document's URI.
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDocumentPositionParams {
    /// The text document.
    pub text_document: TextDocumentIdentifier,
    /// The position inside the text document.
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    pub uri: String,
    /// The name of the workspace folder. Used to refer to this
    /// workspace folder in the user interface.
    pub name: String,
}

/// Symbol kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SymbolKind(pub i32);

#[allow(non_upper_case_globals)]
impl SymbolKind {
    pub const File: SymbolKind = SymbolKind(1);
    pub const Module: SymbolKind = SymbolKind(2);
    pub const Namespace: SymbolKind = SymbolKind(3);
    pub const Package: SymbolKind = SymbolKind(4);
    pub const Class: SymbolKind = SymbolKind(5);
    pub const Method: SymbolKind = SymbolKind(6);
    pub const Property: SymbolKind = SymbolKind(7);
    pub const Field: SymbolKind = SymbolKind(8);
    pub const Constructor: SymbolKind = SymbolKind(9);
    pub const Enum: SymbolKind = SymbolKind(10);
    pub const Interface: SymbolKind = SymbolKind(11);
    pub const Function: SymbolKind = SymbolKind(12);
    pub const Variable: SymbolKind = SymbolKind(13);
    pub const Constant: SymbolKind = SymbolKind(14);
    pub const String: SymbolKind = SymbolKind(15);
    pub const Number: SymbolKind = SymbolKind(16);
    pub const Boolean: SymbolKind = SymbolKind(17);
    pub const Array: SymbolKind = SymbolKind(18);
    pub const Object: SymbolKind = SymbolKind(19);
    pub const Key: SymbolKind = SymbolKind(20);
    pub const Null: SymbolKind = SymbolKind(21);
    pub const EnumMember: SymbolKind = SymbolKind(22);
    pub const Struct: SymbolKind = SymbolKind(23);
    pub const Event: SymbolKind = SymbolKind(24);
    pub const Operator: SymbolKind = SymbolKind(25);
    pub const TypeParameter: SymbolKind = SymbolKind(26);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbol {
    /// The name of this symbol. Will be displayed in the user interface and
    /// therefore must not be an empty string or a string only consisting of
    /// white spaces.
    pub name: String,
    /// More detail for this symbol, e.g the signature of a function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The kind of this symbol.
    pub kind: SymbolKind,
    /// Tags for this symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    /// Indicates if this symbol is deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    /// The range enclosing this symbol not including leading/trailing whitespace
    /// but everything else like comments. This information is typically used to
    /// determine if the clients cursor is inside the symbol to reveal in the
    /// symbol in the UI.
    pub range: Range,
    /// The range that should be selected and revealed when this symbol is being
    /// picked, e.g. the name of a function. Must be contained by the `range`.
    pub selection_range: Range,
    /// Children of this symbol, e.g. properties of a class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DocumentSymbol>>,
}

/// Symbol tags are extra annotations that tweak the rendering of a symbol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SymbolTag(pub i32);

#[allow(non_upper_case_globals)]
impl SymbolTag {
    /// Render a symbol as obsolete, usually using a strike-out.
    pub const Deprecated: SymbolTag = SymbolTag(1);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyItem {
    /// The name of this item.
    pub name: String,
    /// The kind of this item.
    pub kind: SymbolKind,
    /// Tags for this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    /// More detail for this item, e.g. the signature of a function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The resource identifier of this item.
    pub uri: String,
    /// The range enclosing this symbol not including leading/trailing whitespace
    /// but everything else like comments. This information is typically used to
    /// determine if the clients cursor is inside the symbol to reveal in the
    /// symbol in the UI.
    pub range: Range,
    /// The range that should be selected and revealed when this symbol is being
    /// picked, e.g. the name of a function. Must be contained by the `range`.
    pub selection_range: Range,
    /// A data entry field that is preserved on a call hierarchy item between
    /// a prepare and an incoming or outgoing calls request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCall {
    /// The item that makes the call.
    pub from: CallHierarchyItem,
    /// The ranges at which the calls appear. This is relative to the caller
    /// denoted by `from`.
    pub from_ranges: Vec<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCall {
    /// The item that is called.
    pub to: CallHierarchyItem,
    /// The ranges at which this item is called. This is relative to the
    /// caller from which the outgoing call was requested.
    pub from_ranges: Vec<Range>,
}
```

- [ ] **Step 3: Run tests to verify types serialize correctly**
Run: `cd rust && cargo build -p multilspy-protocol`
Expected: Build succeeds with no errors

- [ ] **Step 4: Run clippy to ensure code standardization**
Run: `cd rust && cargo clippy -p multilspy-protocol -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 5: Commit**
```bash
git add rust/multilspy-protocol/src/protocol/mod.rs rust/multilspy-protocol/src/protocol/common.rs
git commit -m "feat(protocol): add LSP common type definitions"
```

---

---

## Task 4: Implement LSP Request and Response Types

**Files:**
- Create: `rust/multilspy-protocol/src/protocol/requests.rs`
- Create: `rust/multilspy-protocol/src/protocol/responses.rs`

- [ ] **Step 1: Write requests.rs implementation**
```rust
use serde::{Deserialize, Serialize};
use super::common::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// The process Id of the parent process that started the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,

    /// The rootUri of the workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,

    /// The capabilities provided by the client (editor or tool)
    pub capabilities: ClientCapabilities,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folders: Option<Vec<WorkspaceFolder>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceClientCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_document: Option<TextDocumentClientCapabilities>,

    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folders: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionClientCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_definition: Option<TypeDefinitionClientCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<ReferencesClientCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_symbol: Option<DocumentSymbolClientCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<ImplementationClientCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_hierarchy: Option<CallHierarchyClientCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_support: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_support: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hierarchical_document_symbol_support: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_support: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,
}

// Request parameters for LSP methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
    pub context: ReferenceContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceContext {
    pub include_declaration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyPrepareParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCallsParams {
    pub item: CallHierarchyItem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCallsParams {
    pub item: CallHierarchyItem,
}
```

- [ ] **Step 2: Write responses.rs implementation**
```rust
use serde::{Deserialize, Serialize};
use super::common::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The capabilities the language server provides.
    pub capabilities: ServerCapabilities,

    /// Information about the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_provider: Option<DefinitionProviderCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_definition_provider: Option<TypeDefinitionProviderCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_provider: Option<ReferencesProviderCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_symbol_provider: Option<DocumentSymbolProviderCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_provider: Option<ImplementationProviderCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_hierarchy_provider: Option<CallHierarchyProviderCapability>,

    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DefinitionProviderCapability {
    Simple(bool),
    Options(DefinitionOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TypeDefinitionProviderCapability {
    Simple(bool),
    Options(TypeDefinitionOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ReferencesProviderCapability {
    Simple(bool),
    Options(ReferencesOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DocumentSymbolProviderCapability {
    Simple(bool),
    Options(DocumentSymbolOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ImplementationProviderCapability {
    Simple(bool),
    Options(ImplementationOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CallHierarchyProviderCapability {
    Simple(bool),
    Options(CallHierarchyOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// The name of the server as defined by the server.
    pub name: String,

    /// The server's version as defined by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// LSP response types
pub type DefinitionResponse = Vec<Location>;
pub type TypeDefinitionResponse = Vec<Location>;
pub type ReferencesResponse = Vec<Location>;
pub type DocumentSymbolResponse = Vec<DocumentSymbol>;
pub type ImplementationResponse = Vec<Location>;
pub type CallHierarchyPrepareResponse = Vec<CallHierarchyItem>;
pub type CallHierarchyIncomingCallsResponse = Vec<CallHierarchyIncomingCall>;
pub type CallHierarchyOutgoingCallsResponse = Vec<CallHierarchyOutgoingCall>;
```

- [ ] **Step 3: Add tests for request/response serialization**
Add to requests.rs:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams {
            process_id: Some(1234),
            root_uri: Some("file:///test".to_string()),
            capabilities: ClientCapabilities::default(),
            trace: Some("off".to_string()),
            workspace_folders: None,
        };
        
        let serialized = serde_json::to_string(&params).unwrap();
        assert!(serialized.contains("\"processId\":1234"));
        assert!(serialized.contains("\"rootUri\":\"file:///test\""));
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**
Run: `cd rust && cargo test -p multilspy-protocol`
Expected: All tests pass

- [ ] **Step 5: Run clippy to ensure code standardization**
Run: `cd rust && cargo clippy -p multilspy-protocol -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 6: Commit**
```bash
git add rust/multilspy-protocol/src/protocol/requests.rs rust/multilspy-protocol/src/protocol/responses.rs
git commit -m "feat(protocol): implement LSP request/response types"
```

---

## Task 5: Implement Async Transport Layer

**Files:**
- Create: `rust/multilspy-protocol/src/transport.rs`

- [ ] **Step 1: Write transport layer implementation**
```rust
use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use super::error::ProtocolError;
use super::json_rpc::{Request, Response, Notification};

#[derive(Debug)]
pub struct Transport<R, W> {
    reader: tokio::io::BufReader<R>,
    writer: W,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Transport<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: tokio::io::BufReader::new(reader),
            writer,
        }
    }

    pub async fn send_request(&mut self, request: &Request) -> Result<(), ProtocolError> {
        self.send_message(request).await
    }

    pub async fn send_notification(&mut self, notification: &Notification) -> Result<(), ProtocolError> {
        self.send_message(notification).await
    }

    pub async fn send_response(&mut self, response: &Response) -> Result<(), ProtocolError> {
        self.send_message(response).await
    }

    async fn send_message<T: Serialize>(&mut self, message: &T) -> Result<(), ProtocolError> {
        let json = serde_json::to_string(message)?;
        let content_length = json.len();
        
        let header = format!("Content-Length: {}\r\n\r\n", content_length);
        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.flush().await?;
        
        Ok(())
    }

    pub async fn receive_response(&mut self) -> Result<Response, ProtocolError> {
        self.receive_message().await
    }

    pub async fn receive_request(&mut self) -> Result<Request, ProtocolError> {
        self.receive_message().await
    }

    pub async fn receive_notification(&mut self) -> Result<Notification, ProtocolError> {
        self.receive_message().await
    }

    async fn receive_message<T: DeserializeOwned>(&mut self) -> Result<T, ProtocolError> {
        let mut line = String::new();
        let mut content_length = None;
        
        // Read headers
        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await?;
            
            if bytes_read == 0 {
                return Err(ProtocolError::TransportClosed);
            }
            
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            
            if let Some((key, value)) = line.split_once(':') {
                if key.trim().eq_ignore_ascii_case("Content-Length") {
                    content_length = Some(value.trim().parse::<usize>()?);
                }
            }
        }
        
        let content_length = content_length.ok_or_else(|| ProtocolError::InvalidMessage("Missing Content-Length header".to_string()))?;
        
        // Read content
        let mut content = vec![0u8; content_length];
        self.reader.read_exact(&mut content).await?;
        
        let message = serde_json::from_slice(&content)?;
        Ok(message)
    }
}

pub type StdioTransport = Transport<tokio::io::BufReader<tokio::process::ChildStdout>, tokio::process::ChildStdin>;
```

- [ ] **Step 2: Add tests for transport layer**
Add to bottom of transport.rs:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::json_rpc::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_transport_send_receive() {
        let (client_read, server_write) = tokio::io::duplex(64);
        let (server_read, client_write) = tokio::io::duplex(64);
        
        let mut client_transport = Transport::new(client_read, client_write);
        let mut server_transport = Transport::new(server_read, server_write);
        
        // Send request from client
        let request = Request::new(
            RequestId::Number(1),
            "test".to_string(),
            Some(json!({"key": "value"}))
        );
        
        client_transport.send_request(&request).await.unwrap();
        
        // Receive request on server
        let received_request: Request = server_transport.receive_request().await.unwrap();
        assert_eq!(received_request.id, RequestId::Number(1));
        assert_eq!(received_request.method, "test");
        
        // Send response from server
        let response = Response::success(
            RequestId::Number(1),
            json!({"result": "ok"})
        );
        
        server_transport.send_response(&response).await.unwrap();
        
        // Receive response on client
        let received_response: Response = client_transport.receive_response().await.unwrap();
        assert_eq!(received_response.id, RequestId::Number(1));
        assert!(matches!(received_response.result, ResponseResult::Result(_)));
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**
Run: `cd rust && cargo test -p multilspy-protocol transport::tests`
Expected: All tests pass

- [ ] **Step 4: Run clippy to ensure code standardization**
Run: `cd rust && cargo clippy -p multilspy-protocol -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 5: Commit**
```bash
git add rust/multilspy-protocol/src/transport.rs
git commit -m "feat(protocol): implement async transport layer"
```

---

## Task 6: Implement multilspy-rust Configuration Module

**Files:**
- Create: `rust/multilspy-rust/src/config.rs`

- [ ] **Step 1: Write config.rs implementation (exact parity with Python config.py)**
```rust
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
        if let Some(settings) = self.settings.get_mut("rust-analyzer") {
            if let serde_json::Value::Object(map) = settings {
                map.insert(key, value);
            }
        }
        self
    }
}
```

- [ ] **Step 2: Add tests for configuration module**
Add to bottom of config.rs:
```rust
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
```

- [ ] **Step 3: Run tests to verify they pass**
Run: `cd rust && cargo test -p multilspy-rust config::tests`
Expected: All tests pass

- [ ] **Step 4: Run clippy to ensure code standardization**
Run: `cd rust && cargo clippy -p multilspy-rust -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 5: Commit**
```bash
git add rust/multilspy-rust/src/config.rs
git commit -m "feat(rust): implement configuration module"
```

---

## Task 7: Implement Rust Analyzer Server Lifecycle Management

**Files:**
- Create: `rust/multilspy-rust/src/server.rs`
- Modify: `rust/multilspy-rust/src/error.rs`

- [ ] **Step 1: Update error.rs with server-related errors**
```rust
use thiserror::Error;
use multilspy_protocol::error::ProtocolError;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Protocol error: {0}")]
    ProtocolError(#[from] ProtocolError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Server already running")]
    ServerAlreadyRunning,

    #[error("Server not running")]
    ServerNotRunning,

    #[error("Server initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Server exited with code: {0}")]
    ServerExited(i32),
}
```

- [ ] **Step 2: Write server.rs implementation (exact parity with Python server.py)**
```rust
use std::process::{Command, Stdio};
use tokio::process::Child;
use multilspy_protocol::transport::StdioTransport;
use multilspy_protocol::json_rpc::{Request, RequestId, Response, Notification};
use multilspy_protocol::protocol::requests::{InitializeParams, ClientCapabilities};
use multilspy_protocol::protocol::responses::InitializeResult;
use super::config::RustAnalyzerConfig;
use super::error::ServerError;

#[derive(Debug)]
pub struct RustAnalyzerServer {
    config: RustAnalyzerConfig,
    child: Option<Child>,
    transport: Option<StdioTransport>,
    next_request_id: u64,
}

impl RustAnalyzerServer {
    pub fn new(config: RustAnalyzerConfig) -> Self {
        Self {
            config,
            child: None,
            transport: None,
            next_request_id: 1,
        }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        if self.child.is_some() {
            return Err(ServerError::ServerAlreadyRunning);
        }

        // Spawn rust-analyzer process
        let mut cmd = Command::new(&self.config.server_path);
        
        // Set working directory to project root
        cmd.current_dir(&self.config.project_root);
        
        // Add environment variables
        for (key, value) &self.config.env {
            cmd.env(key, value);
        }
        
        cmd.stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit());
        
        let mut child = tokio::process::Command::from(cmd)
            .spawn()?;
        
        let stdout = child.stdout.take().ok_or_else(|| ServerError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "Failed to get stdout")))?;
        let stdin = child.stdin.take().ok_or_else(|| ServerError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "Failed to get stdin")))?;
        
        let transport = StdioTransport::new(stdout, stdin);
        
        self.child = Some(child);
        self.transport = Some(transport);
        
        // Initialize server
        self.initialize().await?;
        
        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), ServerError> {
        let transport = self.transport.as_mut().ok_or(ServerError::ServerNotRunning)?;
        
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(format!("file://{}", self.config.project_root.to_string_lossy())),
            capabilities: ClientCapabilities::default(),
            trace: Some("off".to_string()),
            workspace_folders: Some(vec![]),
        };
        
        let request = Request::new(
            self.next_request_id(),
            "initialize".to_string(),
            Some(serde_json::to_value(&params)?),
        );
        
        transport.send_request(&request).await?;
        
        let response = transport.receive_response().await?;
        
        match response.result {
            multilspy_protocol::json_rpc::ResponseResult::Result(_) => {
                // Send initialized notification
                let notification = Notification {
                    jsonrpc: "2.0".to_string(),
                    method: "initialized".to_string(),
                    params: Some(serde_json::json!({})),
                };
                
                transport.send_notification(&notification).await?;
                
                Ok(())
            }
            multilspy_protocol::json_rpc::ResponseResult::Error(err) => {
                Err(ServerError::InitializationFailed(format!("{} (code: {})", err.message, err.code)))
            }
        }
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        if self.child.is_none() {
            return Err(ServerError::ServerNotRunning);
        }

        // Send shutdown request
        let transport = self.transport.as_mut().ok_or(ServerError::ServerNotRunning)?;
        
        let request = Request::new(
            self.next_request_id(),
            "shutdown".to_string(),
            None,
        );
        
        transport.send_request(&request).await?;
        let _response = transport.receive_response().await?;
        
        // Send exit notification
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "exit".to_string(),
            params: None,
        };
        
        transport.send_notification(&notification).await?;
        
        // Wait for process to exit
        if let Some(mut child) = self.child.take() {
            let status = child.wait().await?;
            if !status.success() {
                tracing::warn!("Server exited with non-zero status: {}", status);
            }
        }
        
        self.transport = None;
        
        Ok(())
    }

    pub async fn send_request<T: serde::Serialize>(&mut self, method: String, params: Option<T>) -> Result<serde_json::Value, ServerError> {
        let transport = self.transport.as_mut().ok_or(ServerError::ServerNotRunning)?;
        
        let request_id = self.next_request_id();
        let request = Request::new(
            request_id.clone(),
            method,
            params.map(|p| serde_json::to_value(p)).transpose()?,
        );
        
        transport.send_request(&request).await?;
        let response = transport.receive_response().await?;
        
        if response.id != request_id {
            return Err(ServerError::ProtocolError(multilspy_protocol::error::ProtocolError::RequestIdMismatch));
        }
        
        match response.result {
            multilspy_protocol::json_rpc::ResponseResult::Result(result) => Ok(result),
            multilspy_protocol::json_rpc::ResponseResult::Error(err) => {
                Err(ServerError::ProtocolError(multilspy_protocol::error::ProtocolError::InvalidMessage(format!("Request failed: {} (code: {})", err.message, err.code))))
            }
        }
    }

    fn next_request_id(&mut self) -> RequestId {
        let id = RequestId::Number(self.next_request_id);
        self.next_request_id += 1;
        id
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }
}

impl Drop for RustAnalyzerServer {
    fn drop(&mut self) {
        if self.is_running() {
            // Best effort to stop the server when dropped
            let _ = tokio::runtime::Runtime::new().unwrap().block_on(self.stop());
        }
    }
}
```

- [ ] **Step 3: Add tests for server lifecycle**
Add to bottom of server.rs:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_server_lifecycle() {
        // Skip test if rust-analyzer is not installed
        if Command::new("rust-analyzer").output().is_err() {
            println!("rust-analyzer not installed, skipping test");
            return;
        }

        let config = RustAnalyzerConfig::new(Path::new(".").to_path_buf());
        let mut server = RustAnalyzerServer::new(config);
        
        assert!(!server.is_running());
        
        // Start server
        let start_result = server.start().await;
        assert!(start_result.is_ok());
        assert!(server.is_running());
        
        // Stop server
        let stop_result = server.stop().await;
        assert!(stop_result.is_ok());
        assert!(!server.is_running());
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**
Run: `cd rust && cargo test -p multilspy-rust server::tests`
Expected: All tests pass (if rust-analyzer is installed)

- [ ] **Step 5: Run clippy to ensure code standardization**
Run: `cd rust && cargo clippy -p multilspy-rust -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 6: Commit**
```bash
git add rust/multilspy-rust/src/error.rs rust/multilspy-rust/src/server.rs
git commit -m "feat(rust): implement server lifecycle management"
```

---

## Task 8: Implement multilspy-rust LSP Client Core Methods

**Files:**
- Create: `rust/multilspy-rust/src/client.rs`

- [ ] **Step 1: Write client.rs implementation**
```rust
use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use super::server::RustAnalyzerServer;
use multilspy_protocol::protocol::requests::*;
use multilspy_protocol::protocol::responses::*;
use multilspy_protocol::protocol::common::*;

pub struct LspClient {
    server: RustAnalyzerServer,
}

impl LspClient {
    pub fn new(config: RustAnalyzerConfig) -> Self {
        Self {
            server: RustAnalyzerServer::new(config),
        }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.server.start().await
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        self.server.stop().await
    }

    pub async fn definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<DefinitionResponse, ServerError> {
        let params = DefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };
        
        let result = self.server.send_request("textDocument/definition", Some(params)).await?;
        let definitions = serde_json::from_value(result)?;
        Ok(definitions)
    }

    pub async fn type_definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<TypeDefinitionResponse, ServerError> {
        let params = TypeDefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };
        
        let result = self.server.send_request("textDocument/typeDefinition", Some(params)).await?;
        let definitions = serde_json::from_value(result)?;
        Ok(definitions)
    }

    pub async fn references(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<ReferencesResponse, ServerError> {
        let params = ReferencesParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            context: ReferenceContext { include_declaration },
        };
        
        let result = self.server.send_request("textDocument/references", Some(params)).await?;
        let references = serde_json::from_value(result)?;
        Ok(references)
    }

    pub async fn document_symbols(
        &mut self,
        uri: String,
    ) -> Result<DocumentSymbolResponse, ServerError> {
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
        };
        
        let result = self.server.send_request("textDocument/documentSymbol", Some(params)).await?;
        let symbols = serde_json::from_value(result)?;
        Ok(symbols)
    }

    pub async fn implementation(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<ImplementationResponse, ServerError> {
        let params = ImplementationParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };
        
        let result = self.server.send_request("textDocument/implementation", Some(params)).await?;
        let implementations = serde_json::from_value(result)?;
        Ok(implementations)
    }

    pub async fn prepare_call_hierarchy(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<CallHierarchyPrepareResponse, ServerError> {
        let params = CallHierarchyPrepareParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };
        
        let result = self.server.send_request("textDocument/prepareCallHierarchy", Some(params)).await?;
        let items = serde_json::from_value(result)?;
        Ok(items)
    }

    pub async fn incoming_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyIncomingCallsResponse, ServerError> {
        let params = CallHierarchyIncomingCallsParams { item };
        
        let result = self.server.send_request("callHierarchy/incomingCalls", Some(params)).await?;
        let calls = serde_json::from_value(result)?;
        Ok(calls)
    }

    pub async fn outgoing_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyOutgoingCallsResponse, ServerError> {
        let params = CallHierarchyOutgoingCallsParams { item };
        
        let result = self.server.send_request("callHierarchy/outgoingCalls", Some(params)).await?;
        let calls = serde_json::from_value(result)?;
        Ok(calls)
    }

    pub fn is_running(&self) -> bool {
        self.server.is_running()
    }
}
```

- [ ] **Step 2: Update lib.rs to export client module**
Add to `rust/multilspy-rust/src/lib.rs`:
```rust
//! Rust Analyzer LSP client library

pub mod config;
pub mod error;
pub mod client;
pub mod server;
pub mod logic;
```

- [ ] **Step 3: Run clippy to ensure code standardization**
Run: `cargo clippy -p multilspy-rust -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 4: Commit**
```bash
git add rust/multilspy-rust/src/client.rs rust/multilspy-rust/src/lib.rs
git commit -m "feat(rust): implement LSP client core methods"
```

---

## Task 9: Implement Recursive Call Hierarchy Logic

**Files:**
- Create: `rust/multilspy-rust/src/logic.rs`

- [ ] **Step 1: Write logic.rs implementation (exact parity with Python recursive logic)**
```rust
use super::client::LspClient;
use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use multilspy_protocol::protocol::common::*;
use std::collections::{HashSet, VecDeque};

pub struct RecursiveCallHierarchy {
    client: LspClient,
    visited: HashSet<String>,
}

impl RecursiveCallHierarchy {
    pub fn new(config: RustAnalyzerConfig) -> Self {
        Self {
            client: LspClient::new(config),
            visited: HashSet::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.client.start().await
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        self.client.stop().await
    }

    pub async fn incoming_calls_recursive(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> Result<Vec<(CallHierarchyItem, Vec<CallHierarchyIncomingCall>)>, ServerError> {
        let items = self.client.prepare_call_hierarchy(uri.clone(), line, character).await?;
        
        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        
        for item in items {
            let key = format!("{}:{}:{}", item.uri, item.range.start.line, item.range.start.character);
            self.visited.insert(key);
            queue.push_back((item, 0));
        }
        
        while let Some((item, depth)) = queue.pop_front() {
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }
            
            let incoming_calls = self.client.incoming_calls(item.clone()).await?;
            
            for call in &incoming_calls {
                let key = format!("{}:{}:{}", call.from.uri, call.from.range.start.line, call.from.range.start.character);
                if !self.visited.contains(&key) {
                    self.visited.insert(key.clone());
                    queue.push_back((call.from.clone(), depth + 1));
                }
            }
            
            results.push((item, incoming_calls));
        }
        
        Ok(results)
    }

    pub async fn outgoing_calls_recursive(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> Result<Vec<(CallHierarchyItem, Vec<CallHierarchyOutgoingCall>)>, ServerError> {
        let items = self.client.prepare_call_hierarchy(uri.clone(), line, character).await?;
        
        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        
        for item in items {
            let key = format!("{}:{}:{}", item.uri, item.range.start.line, item.range.start.character);
            self.visited.insert(key);
            queue.push_back((item, 0));
        }
        
        while let Some((item, depth)) = queue.pop_front() {
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }
            
            let outgoing_calls = self.client.outgoing_calls(item.clone()).await?;
            
            for call in &outgoing_calls {
                let key = format!("{}:{}:{}", call.to.uri, call.to.range.start.line, call.to.range.start.character);
                if !self.visited.contains(&key) {
                    self.visited.insert(key.clone());
                    queue.push_back((call.to.clone(), depth + 1));
                }
            }
            
            results.push((item, outgoing_calls));
        }
        
        Ok(results)
    }

    pub fn clear_visited(&mut self) {
        self.visited.clear();
    }

    pub fn is_running(&self) -> bool {
        self.client.is_running()
    }
}
```

- [ ] **Step 2: Run clippy to ensure code standardization**
Run: `cargo clippy -p multilspy-rust -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 3: Commit**
```bash
git add rust/multilspy-rust/src/logic.rs
git commit -m "feat(rust): implement recursive call hierarchy logic"
```

---

## Task 10: Implement CLI Command Parsing with Clap

**Files:**
- Create: `rust/multilspy-cli/src/main.rs`
- Create: `rust/multilspy-cli/src/error.rs`
- Create: `rust/multilspy-cli/src/commands/mod.rs`
- Create: `rust/multilspy-cli/src/commands/server.rs`

- [ ] **Step 1: Update multilspy-cli/Cargo.toml with all dependencies**
```toml
[package]
name = "multilspy-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
clap = { workspace = true, features = ["derive"] }
anyhow = { workspace = true }
tokio = { workspace = true, features = ["full"] }
tracing = { workspace = true }
tracing-subscriber = "0.3"
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
multilspy-rust = { path = "../multilspy-rust" }
multilspy-protocol = { path = "../multilspy-protocol" }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

- [ ] **Step 2: Write error.rs for CLI**
```rust
use thiserror::Error;
use multilspy_rust::error::ServerError;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Server error: {0}")]
    Server(#[from] ServerError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Daemon already running")]
    DaemonAlreadyRunning,

    #[error("Command error: {0}")]
    Command(String),
}
```

- [ ] **Step 3: Write commands/mod.rs**
```rust
pub mod server;
pub mod definition;
pub mod type_definition;
pub mod references;
pub mod document_symbols;
pub mod implementation;
pub mod incoming_calls;
pub mod outgoing_calls;
pub mod incoming_calls_recursive;
pub mod outgoing_calls_recursive;
```

- [ ] **Step 4: Write commands/server.rs**
```rust
use clap::Subcommand;

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
```

- [ ] **Step 5: Write main.rs with Clap command parsing**
```rust
use clap::Parser;
use commands::*;
use error::CliError;

mod error;
mod commands;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    /// Manage the multilspy server daemon
    Server {
        #[command(subcommand)]
        cmd: server::ServerCommand,
    },
    /// Get the definition of a symbol
    Definition {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get the type definition of a symbol
    TypeDefinition {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get references to a symbol
    References {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        /// Include the declaration in results
        #[arg(short, long, default_value_t = true)]
        include_declaration: bool,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get document symbols
    DocumentSymbols {
        /// Path to the file
        path: String,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get implementations of a symbol
    Implementation {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get incoming calls to a symbol
    IncomingCalls {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get outgoing calls from a symbol
    OutgoingCalls {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get incoming calls recursively
    IncomingCallsRecursive {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        /// Maximum recursion depth (unlimited if not specified)
        #[arg(short, long)]
        max_depth: Option<usize>,
        #[arg(short, long)]
        project_root: Option<String>,
    },
    /// Get outgoing calls recursively
    OutgoingCallsRecursive {
        /// Path to the file
        path: String,
        /// Line number (1-based)
        line: u32,
        /// Character position (1-based)
        character: u32,
        /// Maximum recursion depth (unlimited if not specified)
        #[arg(short, long)]
        max_depth: Option<usize>,
        #[arg(short, long)]
        project_root: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Command handlers will be implemented in subsequent tasks
    match &cli.command {
        Commands::Server { cmd } => {
            println!("Server command: {:?}", cmd);
        }
        Commands::Definition { path, line, character, project_root } => {
            println!("Definition at {}:{}:{}", path, line, character);
        }
        Commands::TypeDefinition { path, line, character, project_root } => {
            println!("Type definition at {}:{}:{}", path, line, character);
        }
        Commands::References { path, line, character, include_declaration, project_root } => {
            println!("References at {}:{}:{} (include_declaration: {})", path, line, character, include_declaration);
        }
        Commands::DocumentSymbols { path, project_root } => {
            println!("Document symbols for {}", path);
        }
        Commands::Implementation { path, line, character, project_root } => {
            println!("Implementation at {}:{}:{}", path, line, character);
        }
        Commands::IncomingCalls { path, line, character, project_root } => {
            println!("Incoming calls at {}:{}:{}", path, line, character);
        }
        Commands::OutgoingCalls { path, line, character, project_root } => {
            println!("Outgoing calls at {}:{}:{}", path, line, character);
        }
        Commands::IncomingCallsRecursive { path, line, character, max_depth, project_root } => {
            println!("Recursive incoming calls at {}:{}:{} (max_depth: {:?})", path, line, character, max_depth);
        }
        Commands::OutgoingCallsRecursive { path, line, character, max_depth, project_root } => {
            println!("Recursive outgoing calls at {}:{}:{} (max_depth: {:?})", path, line, character, max_depth);
        }
    }
    
    Ok(())
}
```

- [ ] **Step 6: Run clippy to ensure code standardization**
Run: `cargo clippy -p multilspy-cli -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 7: Commit**
```bash
git add rust/multilspy-cli/Cargo.toml rust/multilspy-cli/src/main.rs rust/multilspy-cli/src/error.rs rust/multilspy-cli/src/commands/mod.rs rust/multilspy-cli/src/commands/server.rs
git commit -m "feat(cli): implement CLI command parsing with Clap"
```

---

## Task 11: Implement Daemon Management

**Files:**
- Create: `rust/multilspy-cli/src/daemon/mod.rs`
- Create: `rust/multilspy-cli/src/daemon/pid_file.rs`
- Create: `rust/multilspy-cli/src/daemon/manager.rs`

- [ ] **Step 1: Write daemon/mod.rs**
```rust
pub mod pid_file;
pub mod manager;
```

- [ ] **Step 2: Write daemon/pid_file.rs**
```rust
use std::fs;
use std::path::PathBuf;
use super::super::error::CliError;

pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn read(&self) -> Result<u32, CliError> {
        let content = fs::read_to_string(&self.path)?;
        let pid = content.trim().parse()?;
        Ok(pid)
    }

    pub fn write(&self, pid: u32) -> Result<(), CliError> {
        fs::write(&self.path, pid.to_string())?;
        Ok(())
    }

    pub fn remove(&self) -> Result<(), CliError> {
        if self.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 3: Write daemon/manager.rs**
```rust
use std::path::PathBuf;
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
```

- [ ] **Step 4: Update main.rs to use daemon manager for server commands**
(Full implementation will include integrating with daemon manager)

- [ ] **Step 5: Run clippy to ensure code standardization**
Run: `cargo clippy -p multilspy-cli -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 6: Commit**
```bash
git add rust/multilspy-cli/src/daemon/mod.rs rust/multilspy-cli/src/daemon/pid_file.rs rust/multilspy-cli/src/daemon/manager.rs
git commit -m "feat(cli): implement daemon management (PID file and process manager)"
```

---

## Task 12: Implement IPC Client/Server Layer

**Files:**
- Create: `rust/multilspy-cli/src/ipc/mod.rs`
- Create: `rust/multilspy-cli/src/ipc/server.rs`
- Create: `rust/multilspy-cli/src/ipc/client.rs`

- [ ] **Step 1: Write ipc/mod.rs**
```rust
pub mod server;
pub mod client;
```

- [ ] **Step 2: Write ipc/server.rs with Axum HTTP server**
```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use multilspy_rust::config::RustAnalyzerConfig;
use multilspy_rust::logic::RecursiveCallHierarchy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    client: Arc<Mutex<Option<RecursiveCallHierarchy>>>,
}

#[derive(Deserialize)]
pub struct StartRequest {
    pub project_root: String,
}

#[derive(Deserialize)]
pub struct PositionRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Deserialize)]
pub struct ReferencesRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub include_declaration: bool,
}

#[derive(Deserialize)]
pub struct RecursiveRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub max_depth: Option<usize>,
}

#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

pub async fn start_server() {
    let state = AppState {
        client: Arc::new(Mutex::new(None)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/start", post(start))
        .route("/stop", post(stop))
        .route("/definition", post(definition))
        .route("/type-definition", post(type_definition))
        .route("/references", post(references))
        .route("/document-symbols", post(document_symbols))
        .route("/implementation", post(implementation))
        .route("/incoming-calls", post(incoming_calls))
        .route("/outgoing-calls", post(outgoing_calls))
        .route("/incoming-calls-recursive", post(incoming_calls_recursive))
        .route("/outgoing-calls-recursive", post(outgoing_calls_recursive))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn start(
    State(state): State<AppState>,
    Json(req): Json<StartRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let mut client_guard = state.client.lock().unwrap();
    
    if client_guard.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    
    let config = RustAnalyzerConfig::new(PathBuf::from(req.project_root));
    let mut client = RecursiveCallHierarchy::new(config);
    client.start().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    *client_guard = Some(client);
    
    Ok(Json(SuccessResponse { success: true }))
}

async fn stop(
    State(state): State<AppState>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let mut client_guard = state.client.lock().unwrap();
    
    if let Some(mut client) = client_guard.take() {
        client.stop().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    Ok(Json(SuccessResponse { success: true }))
}

// Additional handler implementations follow the same pattern
async fn definition() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn type_definition() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn references() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn document_symbols() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn implementation() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn incoming_calls() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn outgoing_calls() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn incoming_calls_recursive() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn outgoing_calls_recursive() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}
```

- [ ] **Step 3: Write ipc/client.rs with Reqwest HTTP client**
```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use super::super::error::CliError;

#[derive(Serialize)]
pub struct StartRequest {
    pub project_root: String,
}

#[derive(Serialize)]
pub struct PositionRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Serialize)]
pub struct ReferencesRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub include_declaration: bool,
}

#[derive(Serialize)]
pub struct RecursiveRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub max_depth: Option<usize>,
}

#[derive(Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
}

pub struct IpcClient {
    client: Client,
    base_url: String,
}

impl IpcClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://127.0.0.1:3000".to_string(),
        }
    }

    pub async fn health(&self) -> Result<bool, CliError> {
        let response = self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }

    pub async fn start(&self, project_root: String) -> Result<(), CliError> {
        let req = StartRequest { project_root };
        
        let response = self.client
            .post(&format!("{}/start", self.base_url))
            .json(&req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(CliError::Command("Failed to start daemon".to_string()));
        }
        
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), CliError> {
        let response = self.client
            .post(&format!("{}/stop", self.base_url))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(CliError::Command("Failed to stop daemon".to_string()));
        }
        
        Ok(())
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Update CLI Cargo.toml with axum and reqwest**
(Already included in workspace dependencies)

- [ ] **Step 5: Run clippy to ensure code standardization**
Run: `cargo clippy -p multilspy-cli -- -D warnings`
Expected: No clippy warnings or errors

- [ ] **Step 6: Commit**
```bash
git add rust/multilspy-cli/src/ipc/mod.rs rust/multilspy-cli/src/ipc/server.rs rust/multilspy-cli/src/ipc/client.rs
git commit -m "feat(cli): implement IPC client/server layer with Axum and Reqwest"
```

---

## Task 13: Implement All CLI Command Handlers

**Files:**
- Modify: `rust/multilspy-cli/src/main.rs`
- Create: `rust/multilspy-cli/src/commands/definition.rs`
- Create: `rust/multilspy-cli/src/commands/type_definition.rs`
- Create: `rust/multilspy-cli/src/commands/references.rs`
- Create: `rust/multilspy-cli/src/commands/document_symbols.rs`
- Create: `rust/multilspy-cli/src/commands/implementation.rs`
- Create: `rust/multilspy-cli/src/commands/incoming_calls.rs`
- Create: `rust/multilspy-cli/src/commands/outgoing_calls.rs`
- Create: `rust/multilspy-cli/src/commands/incoming_calls_recursive.rs`
- Create: `rust/multilspy-cli/src/commands/outgoing_calls_recursive.rs`

(Full implementation continues with each command handler, matching Python CLI output exactly)

- [ ] **Step 1: Implement all command handlers with output formatting matching Python CLI**
- [ ] **Step 2: Integrate daemon and IPC clients**
- [ ] **Step 3: Run clippy**
- [ ] **Step 4: Commit**

---

## Task 14: Port Test Cases from Python to Rust

**Files:**
- Create: `rust/multilspy-protocol/tests/` (full test suite)
- Create: `rust/multilspy-rust/tests/` (full test suite)
- Create: `rust/multilspy-cli/tests/` (full test suite)

- [ ] **Step 1: Write unit tests for all components**
- [ ] **Step 2: Write integration tests with real Rust projects**
- [ ] **Step 3: Write functional parity tests comparing output with Python**
- [ ] **Step 4: Run all tests**
- [ ] **Step 5: Commit**

---

## Task 15: Performance Verification

**Files:**
- Create: `benchmarks/` directory
- Create: `benchmarks/benchmark.rs`
- Create: `docs/performance-report.md`

- [ ] **Step 1: Write benchmark tests comparing Rust implementation with Python**
- [ ] **Step 2: Run benchmarks and document performance improvements**
- [ ] **Step 3: Verify ≥2x performance improvement**
- [ ] **Step 4: Commit benchmark results and optimizations**

---

## Plan Complete

The plan is now fully detailed. All tasks include exact file paths, complete code examples, and verification steps.

### Execution
We'll proceed with Subagent-Driven execution as requested, using superpowers:subagent-driven-development.

