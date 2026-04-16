# Multilspy Rust CLI

A high-performance, daemon-based LSP CLI for Rust Analyzer, optimized for AI agent usage.

## Overview

This Rust implementation provides a fast, reliable interface to the Rust Analyzer language server, designed specifically for use by AI coding agents and other tools that need to make repeated LSP queries. Unlike the Python library which starts a new language server process for each session, the Rust CLI uses a persistent daemon architecture that keeps the language server running in the background between requests, drastically reducing overhead for repeated queries.

## Crate Structure

The Rust implementation is split into three crates:

### `multilspy-protocol`
Core LSP protocol and JSON-RPC implementation. Handles:
- LSP type definitions
- JSON-RPC message serialization/deserialization
- Error types and handling
- Transport layer abstraction

### `multilspy-rust`
Rust Analyzer LSP client library. Implements:
- LSP client logic for communicating with Rust Analyzer
- Core LSP request handlers (definition, references, symbols, etc.)
- Advanced features like recursive call hierarchy and dependency graph analysis
- Server lifecycle management

### `multilspy-cli`
Command-line interface application. Provides:
- User-friendly CLI commands for all LSP operations
- Persistent daemon management
- IPC communication between CLI clients and the daemon
- Structured JSON output for all commands

## Installation

### Prerequisites
- Rust 1.75+ (with `cargo`)
- Rust Analyzer installed and available in `PATH` (or specify path in initialize params)

### Build from source
```bash
cd rust
cargo build --release
```

The binary will be available at `target/release/ra-lsp`. You can install it to your system with:
```bash
cargo install --path multilspy-cli
```

## Usage

### Global Options
- `-w, --workspace <PATH>`: Workspace root directory (defaults to current directory)
- `-i, --initialize-params <PATH>`: Path to `ra_initialize_params.json` file
- `-t, --wait-work-done-progress-create-max-time <SECONDS>`: Maximum wait time for Rust Analyzer work done progress

### Initialize Params Configuration
The CLI uses a `ra_initialize_params.json` file to configure Rust Analyzer. This file follows the standard LSP InitializeParams structure. The CLI will look for this file in the workspace root by default, or you can specify a custom path with the `-i` option or `RA_LSP_INIT_PARAMS_PATH` environment variable.

If both the environment variable and `-i` option are provided, the configuration will be merged, with the `-i` option taking precedence.

### Commands

#### `definition`
Get the definition location of a symbol at a given position.
```bash
ra-lsp definition --relative-path src/main.rs --line 10 --character 5
```

Output:
```json
{
  "result": [
    {
      "uri": "file:///workspace/src/main.rs",
      "range": {
        "start": { "line": 24, "character": 4 },
        "end": { "line": 24, "character": 16 }
      }
    }
  ]
}
```

#### `type-definition`
Get the type definition location of a symbol at a given position.
```bash
ra-lsp type-definition --relative-path src/main.rs --line 10 --character 5
```

#### `implementation`
Get the implementation locations of a trait or interface at a given position.
```bash
ra-lsp implementation --relative-path src/main.rs --line 10 --character 5
```

#### `references`
Find all references to a symbol at a given position.
```bash
ra-lsp references --relative-path src/main.rs --line 10 --character 5 --include-declaration true
```

Options:
- `--include-declaration <BOOLEAN>`: Include the declaration location in results (default: true)

#### `document-symbols`
List all symbols in a document.
```bash
ra-lsp document-symbols --relative-path src/main.rs
```

#### `workspace-symbols`
Search for symbols across the entire workspace.
```bash
ra-lsp workspace-symbols --query "function_name" --limit 10
```

Options:
- `--query <STRING>`: Search query (required)
- `--limit <NUMBER>`: Maximum number of results to return (optional)

#### `workspace-symbol-resolve`
Resolve additional fields for a workspace symbol returned from `workspace-symbols`.
```bash
ra-lsp workspace-symbol-resolve --symbol-json '{"name": "function_name", "kind": 12, "location": {"uri": "file:///workspace/src/main.rs", "range": {"start": {"line": 0, "character": 0}, "end": {"line": 10, "character": 0}}}, "containerName": "main"}'
```

Options:
- `--symbol-json <JSON>`: Workspace symbol JSON object
- `--symbol-file <PATH>`: Path to file containing workspace symbol JSON

#### `incoming-calls`
Find all incoming calls to a function at a given position.
```bash
ra-lsp incoming-calls --relative-path src/main.rs --line 10 --character 5
```

#### `outgoing-calls`
Find all outgoing calls from a function at a given position.
```bash
ra-lsp outgoing-calls --relative-path src/main.rs --line 10 --character 5
```

#### `incoming-calls-recursive`
Find all incoming calls to a function recursively, traversing the call graph up to a maximum depth.
```bash
ra-lsp incoming-calls-recursive --relative-path src/main.rs --line 10 --character 5 --max-depth 3
```

Options:
- `--max-depth <NUMBER>`: Maximum recursion depth (optional, default: unlimited)

#### `outgoing-calls-recursive`
Find all outgoing calls from a function recursively, traversing the call graph up to a maximum depth.
```bash
ra-lsp outgoing-calls-recursive --relative-path src/main.rs --line 10 --character 5 --max-depth 3
```

Options:
- `--max-depth <NUMBER>`: Maximum recursion depth (optional, default: unlimited)

#### `analyze-func-deps-graph`
Analyze dependencies between functions implementing specified traits, or between specific target functions.

Analyze trait implementations:
```bash
ra-lsp analyze-func-deps-graph MyTrait --target-dir src --target-dir tests
```

Analyze specific functions:
```bash
ra-lsp analyze-func-deps-graph --function-target src/main.rs,10,5 --function-target src/lib.rs,20,3
```

Use target specs with custom metadata:
```bash
ra-lsp analyze-func-deps-graph --target-spec '{"fn_type":"trait_impl","trait_name":"MyTrait","target_dir":"src","extra":{"label":"core"}}'
```

Options:
- `<TRAIT>...`: Trait names to analyze (repeat for multiple traits)
- `--target-dir <DIR>`: Target directories to search for trait implementations (repeat for multiple directories, required when traits are provided)
- `--function-target <PATH,LINE,CHARACTER>`: Specific functions to analyze (repeat for multiple functions)
- `--target-spec <JSON>`: Generic target specifications with optional extra metadata (repeat for multiple targets)

#### `status`
Check the status of the daemon for the current workspace.
```bash
ra-lsp status
```

Output:
```json
{
  "result": {
    "workspace": "/workspace/project",
    "pid": 4242,
    "port": 53741,
    "uptime_secs": 18
  }
}
```

#### `stop`
Stop the daemon for the current workspace.
```bash
ra-lsp stop
```

Output:
```json
{
  "result": "shutdown_ack"
}
```

## Features

- **Persistent Daemon**: The Rust Analyzer server runs in the background as a daemon, eliminating startup overhead for repeated queries
- **Structured JSON Output**: All commands return consistent JSON output with either a `result` field on success or an `error` field on failure
- **Advanced LSP Features**: Supports standard LSP operations plus recursive call hierarchy and function dependency graph analysis
- **Flexible Configuration**: Supports mergeable initialize params from files and environment variables
- **Automatic Daemon Management**: The daemon is automatically started on first request and stopped when idle (configurable)
- **Fast Performance**: Written in Rust with minimal overhead, providing sub-100ms response times for most queries after initial server startup

## Development

### Build
```bash
cargo build
```

### Test
```bash
cargo test
```

### Run
```bash
cargo run -- <command> [options]
```

### Lint
```bash
cargo clippy
```

### Format
```bash
cargo fmt
```
