# Multilspy Rust Port Design Document
**Date**: 2026-04-11
**Status**: Approved

## Overview
This document describes the design for porting the Python-based `multilspy` LSP client library to Rust. The port will deliver significantly better performance, lower memory footprint, and easier distribution while maintaining full functional parity with the existing Python implementation for Rust Analyzer support.

## Success Criteria
✅ All functionality from the Python implementation is ported and works identically
✅ CLI commands produce exactly the same output as the Python version for identical inputs
✅ All ported test cases pass
✅ No panics in production code paths
✅ Async implementation delivers at least 2x throughput improvement over the Python implementation for concurrent LSP requests

## Architecture
We're using a modular layered architecture with 3 distinct crates in a Cargo workspace:

### Crate Structure
```
rust/
├── Cargo.toml              # Workspace configuration
├── multilspy-protocol/     # Core LSP protocol crate
│   ├── Cargo.toml
│   └── src/
│       ├── json_rpc/       # JSON-RPC 2.0 message parsing/serialization
│       ├── protocol/       # LSP protocol type definitions (from LSP spec)
│       ├── transport/      # Async stdio/IPC transport layer
│       └── error.rs        # Core error types
├── multilspy-rust/         # Core Rust Analyzer library crate
│   ├── Cargo.toml
│   └── src/
│       ├── config.rs       # Configuration handling (full parity with Python)
│       ├── server.rs       # Rust Analyzer server lifecycle management
│       ├── client.rs       # LSP client implementation with all request methods
│       ├── logic.rs        # Complex higher-level logic (recursive call hierarchy)
│       └── error.rs        # Library-specific error types
└── multilspy-cli/          # CLI crate
    ├── Cargo.toml
    └── src/
        ├── commands/       # Individual CLI command implementations
        ├── daemon/         # Background server daemon management
        ├── ipc/            # IPC communication protocol
        └── main.rs         # CLI entry point
```

### Technology Stack
- Tokio: Async runtime for all I/O operations
- Tracing: Structured logging
- Clap: CLI argument parsing
- Serde: JSON serialization/deserialization (with serde_json for JSON handling)
- Thiserror: Custom error type implementation for library-specific errors
- Anyhow: Easy error handling for application code (CLI layer)
- Reqwest: HTTP client (for future use if needed)
- Axum: HTTP server (for future use if needed)

## Core Components

### 1. multilspy-protocol crate
- **JSON-RPC Layer**: Implements parsing and serialization of JSON-RPC 2.0 messages following LSP specification. Handles request ID generation and correlation.
- **Protocol Types**: Strongly typed Rust structs for all required LSP messages and types, generated from official LSP JSON schema for accuracy.
- **Transport Layer**: Async implementation of LSP communication over stdio (for direct server communication) and Unix domain sockets/Windows named pipes (for daemon IPC). Handles message frame parsing with Content-Length headers.
- **Error Types**: Generic protocol errors (parse errors, invalid messages, transport errors).

### 2. multilspy-rust crate
- **Config**: Strongly typed configuration structure for Rust Analyzer with full parity to Python implementation options. Supports loading from environment variables and explicit configuration objects.
- **Server**: Manages Rust Analyzer subprocess lifecycle: spawning from PATH, handling stdio pipes, initialize handshake, state management, and proper cleanup on drop.
- **Client**: Implements public async API with all LSP request methods:
  - `definition()`, `type_definition()`, `references()`
  - `document_symbols()`, `implementation()`
  - `incoming_calls()`, `outgoing_calls()`
  - Handles request cancellation and timeout logic
- **Logic**: Implements higher-level functionality on top of base LSP requests:
  - `incoming_calls_recursive()` with configurable max depth
  - `outgoing_calls_recursive()` with configurable max depth

### 3. multilspy-cli crate
- **Command Parsing**: Exact parity with all Python CLI commands and arguments:
  - **Global flags**: `--project <PATH>`
  - **Server commands**: `server start [--no-daemon]`, `server stop`, `server status`
  - **LSP commands**: `definition`, `type-definition`, `references`, `document-symbols`, `implementation`, `incoming-calls`, `outgoing-calls`, `incoming-calls-recursive`, `outgoing-calls-recursive`
  - All line/column numbers are 1-based, file path resolution matches Python behavior
- **Daemon Management**: Implements background server lifecycle: PID file management, IPC socket creation, daemon process spawning and shutdown.
- **IPC Layer**: Handles communication between CLI and daemon over Unix domain sockets/Windows named pipes using JSON-RPC protocol.
- **Output Formatting**: Exactly matches Python CLI JSON output format:
  - Success: `{"status": "ok", "result": [...]}`
  - Error: `{"status": "error", "message": "error text"}`

## Data Flow

### Single-shot command flow
1. User runs CLI command
2. CLI parses arguments and resolves project path
3. CLI creates new `multilspy_rust::Server` instance
4. Server spawns rust-analyzer subprocess, performs initialize handshake
5. Client sends requested LSP command
6. Server returns response, CLI formats as JSON and prints
7. Server sends shutdown request, waits for process exit
8. CLI exits

### Background daemon flow
1. User runs `ra-lsp server start`
2. CLI spawns background daemon process, creates PID file and IPC socket
3. Daemon initializes rust-analyzer and waits for connections
4. User runs command, CLI connects to daemon socket
5. CLI sends request over IPC, daemon forwards to LSP server
6. Daemon returns response to CLI over IPC
7. CLI formats response as JSON and prints
8. On `ra-lsp server stop`, daemon cleans up resources and exits

## Error Handling
- **Zero panics in production code**: All potential panics are converted to proper error types
- **Layered error handling**: Protocol errors, library errors, and CLI errors are handled separately
- **Exact error parity**: Error messages and format exactly match Python CLI output
- All errors implement `std::error::Error` for interoperability

## Testing Strategy
1. **Unit tests**: Protocol parsing, configuration handling, path resolution, recursive logic tests
2. **Integration tests**: End-to-end tests using existing Python test suite project, all LSP method parity tests, daemon lifecycle tests, CLI output comparison tests
3. **Performance tests**: Throughput tests (verify ≥2x improvement over Python), memory footprint tests

## Implementation Workflow
1. Create workspace and crate structure
2. Port multilspy-protocol layer first, validate with unit tests
3. Port multilspy-rust core library, validate with integration tests
4. Port multilspy-cli implementation, validate end-to-end
5. Port test cases from Python test suite to Rust
6. Run performance tests to verify success criteria