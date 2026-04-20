# Multilspy Rust CLI

`multilspy-cli` is the Rust command-line frontend for the Multilspy workspace. It wraps the `multilspy-rust` client library in a daemon-backed CLI so repeated LSP requests can reuse one background Rust Analyzer session per workspace.

This README is intentionally scoped to the current `multilspy-cli` implementation.

## Workspace Layout

This Rust workspace currently contains three crates:

- `multilspy-protocol`: shared protocol and serialization types.
- `multilspy-rust`: the Rust Analyzer client implementation.
- `multilspy-cli`: the CLI binary and daemon lifecycle layer documented here.

## Executable Naming

The compiled executable is named `ra-lsp`:

```bash
cargo build --release -p multilspy-cli
ls target/release/ra-lsp
```

Install it with:

```bash
cargo install --path multilspy-cli
```

Use `ra-lsp` in shell commands and scripts.

One current implementation detail is worth knowing:

- `--help` usage renders as `ra-lsp`.
- `--version` currently prints `multilspy <version>` because clap metadata sets the command name to `multilspy`.

## What The CLI Does

`ra-lsp` exposes these implemented commands:

- `definition`
- `type-definition`
- `implementation`
- `references`
- `document-symbols`
- `workspace-symbols`
- `workspace-symbol-resolve`
- `incoming-calls`
- `outgoing-calls`
- `incoming-calls-recursive`
- `outgoing-calls-recursive`
- `analyze-func-deps-graph`
- `status`
- `stop`

There is also an internal hidden `daemon` subcommand used only for auto-spawn. It is not intended for normal manual use.

## Global Options

All public commands accept these global options:

- `-w, --workspace <PATH>`: workspace root directory. Defaults to the current working directory if omitted.
- `-i, --initialize-params <PATH>`: path to an initialize params JSON file. Example config json file: [`./multilspy-rust/ra_initialize_params.json`](./multilspy-rust/ra_initialize_params.json).
- `-t, --wait-work-done-progress-create-max-time <SECONDS>`: optional maximum wait time passed through to Rust Analyzer setup for `workDoneProgress` creation.

## Initialize Params And Precedence

The CLI always resolves an initialize params file before it talks to the daemon, including for `status` and `stop`.

Resolution order:

1. If `RA_LSP_INIT_PARAMS_PATH` is set and `--initialize-params` is also passed, the two JSON files are merged.
2. In that merged case, `--initialize-params` overrides matching JSON fields recursively.
3. If only `RA_LSP_INIT_PARAMS_PATH` is set, that file is used.
4. If only `--initialize-params` is passed, that file is used.
5. If neither is set, the CLI falls back to `<workspace>/ra_initialize_params.json`.

Important details:

- Each resolved file must exist and contain valid JSON.
- The JSON must deserialize as an LSP `InitializeParams` structure.
- When env and CLI inputs are merged, the CLI writes a temporary merged JSON file under the system temp directory and removes it after the client process exits.
- Object values are merged recursively.
- Non-object values are replaced by the CLI-provided overlay value.

Example:

```bash
export RA_LSP_INIT_PARAMS_PATH=/tmp/base-init.json
ra-lsp --initialize-params ./ra_initialize_params.override.json status
```

## File Input: `--uri` Vs `--relative-path`

Document-oriented commands require exactly one file input:

- `--uri <file://...>`
- `--relative-path <PATH>`

This applies to:

- `definition`
- `type-definition`
- `implementation`
- `references`
- `document-symbols`
- `incoming-calls`
- `outgoing-calls`
- `incoming-calls-recursive`
- `outgoing-calls-recursive`

Behavior differences:

- `--uri` is passed through as provided.
- `--relative-path` is resolved against the current working directory, not against `--workspace`.
- Relative paths are canonicalized and converted into escaped `file://` URIs.
- A missing or inaccessible `--relative-path` fails before any LSP request is sent.
- Passing both flags together is rejected.

Example:

```bash
cd /path/to/workspace
ra-lsp definition --relative-path src/main.rs --line 35 --character 12
```

## Positions Are Zero-Based

All line and character positions accepted by the CLI are zero-based.

Examples:

- `--line 0 --character 0` means the first character of the first line.
- In the bundled `multilspy-rust/test-rust-project/src/main.rs`, the call to `create_hello("world")` in `helper()` is addressed as `--line 35 --character 12`.

## JSON Output Envelope

For operational subcommands that reach the IPC layer, the CLI writes one JSON object to `stdout`.

Notable exceptions:

- `--help` and `--version` print clap output, not the JSON envelope.
- clap parse/usage errors are not emitted in the JSON envelope.
- `stop` returns success with no JSON output when no daemon exists for the selected workspace.

Success shape:

```json
{
  "result": {}
}
```

Error shape:

```json
{
  "error": {
    "code": -32602,
    "message": "..."
  }
}
```

The envelope is defined by `IpcResponse`:

- Success responses set `result` and omit `error`.
- Error responses set `error` and omit `result`.
- A non-zero process exit code is used when the command fails.

Error codes currently used by the CLI:

- `-32602`: invalid params or local input/validation failures.
- `-32603`: internal serialization or IPC-level internal error.
- `-32601`: unknown IPC method.
- `-32000`: LSP request failed after reaching the daemon.
- `-1`: daemon startup or top-level daemon process failure.

## Daemon Lifecycle

The CLI uses one daemon process per canonical workspace path.

### Auto-spawn And Reuse

- Normal client commands call `ensure_daemon(...)` before sending the request.
- If a healthy daemon already exists for the workspace, the CLI reuses it.
- If the pidfile exists but the process is dead or does not answer `ping`, the CLI removes the stale pidfile and starts a fresh daemon.
- The daemon process binds to `127.0.0.1` on an ephemeral port and serves requests on `POST /rpc`.

### Workspace Tracking

- Daemon metadata is stored in a pidfile under the system temp directory in `multilspy-cli/`.
- The pidfile name is derived from a hash of the canonical workspace path.
- Stored metadata includes the daemon `pid`, `port`, and workspace path.

### Startup Wait

- After spawn, the client polls for daemon readiness.
- The current implementation waits in 10-second intervals for up to 120 attempts.
- If the daemon never becomes reachable, the client errors with `failed to start daemon within 1200 seconds`.

### Idle Shutdown

- The daemon tracks its last request time.
- If it stays idle for 2 hours, it shuts itself down.
- The inactivity check runs every 30 seconds.

### `status`

`status` reports daemon metadata for the selected workspace:

```bash
ra-lsp --workspace /path/to/workspace --initialize-params /path/to/ra_initialize_params.json status
```

Response shape:

```json
{
  "result": {
    "workspace": "/path/to/workspace",
    "pid": 4242,
    "port": 53741,
    "uptime_secs": 18
  }
}
```

Behavior note:

- `status` is a normal client command, so it auto-spawns the daemon if one is not already running.

### `stop`

`stop` shuts down the daemon for the selected workspace:

```bash
ra-lsp --workspace /path/to/workspace --initialize-params /path/to/ra_initialize_params.json stop
```

Successful response:

```json
{
  "result": "shutdown_ack"
}
```

Special cases:

- If no daemon exists, `stop` does not start one just to stop it.
- In that no-daemon case, the process exits successfully without printing a JSON response.
- Initialize params resolution still happens before the daemon lookup short-circuit.

## Command Reference

### `definition`

Go to the definition at a zero-based position.

```bash
ra-lsp definition \
  --relative-path src/main.rs \
  --line 35 \
  --character 12
```

Current success payload:

- `{ "result": Location[] }`
- Empty matches return `{ "result": [] }`

Example result shape:

```json
{
  "result": [
    {
      "uri": "file:///workspace/src/main.rs",
      "range": {
        "start": { "line": 24, "character": 0 },
        "end": { "line": 28, "character": 1 }
      }
    }
  ]
}
```

### `type-definition`

Go to the type definition at a zero-based position.

```bash
ra-lsp type-definition \
  --relative-path src/main.rs \
  --line 35 \
  --character 8
```

Current success payload:

- `{ "result": Location[] }`

### `implementation`

Find implementations for the symbol at a zero-based position.

```bash
ra-lsp implementation \
  --relative-path src/main.rs \
  --line 0 \
  --character 6
```

Current success payload:

- `{ "result": Location[] }`

For example, on the `Greeter` trait in the test project this returns implementation locations for both `Hello` and `Goodbye`.

### `references`

Find references at a zero-based position.

```bash
ra-lsp references \
  --relative-path src/main.rs \
  --line 24 \
  --character 5 \
  --include-declaration false
```

Flags:

- `--include-declaration <BOOL>`: defaults to `true`.

Current success payload:

- `{ "result": Location[] }`

### `document-symbols`

List document symbols for one file.

```bash
ra-lsp document-symbols --relative-path src/main.rs
```

Current success payload:

- `{ "result": DocumentSymbol[] }`

Notes:

- Symbols may contain nested `children`.
- In the test project, top-level results include symbols such as `Greeter`, `Hello`, `Goodbye`, `create_hello`, `call_greet`, `helper`, and `main`.

### `workspace-symbols`

Search workspace-wide symbols by query string.

```bash
ra-lsp workspace-symbols --query helper
```

Optional limit:

```bash
ra-lsp workspace-symbols --query e --limit 10
```

Current validation and behavior:

- `--query` must not be empty or whitespace-only.
- `--limit`, when provided, must be greater than zero.
- The CLI truncates the returned list after the server response.

Current success payload:

- `{ "result": WorkspaceSymbol[] | SymbolInformation[] }`

### `workspace-symbol-resolve`

Resolve additional fields for one symbol returned by `workspace-symbols`.

Use exactly one of:

- `--symbol-json <JSON>`
- `--symbol-file <PATH>`

Example:

```bash
ra-lsp workspace-symbol-resolve \
  --symbol-json '{"name":"helper","kind":12,"location":{"uri":"file:///workspace/src/main.rs","range":{"start":{"line":34,"character":0},"end":{"line":37,"character":1}}},"containerName":"main"}'
```

Current success payload:

- `{ "result": WorkspaceSymbol | null }`

Notes:

- The input must be one JSON object, not an array.
- The CLI accepts either `WorkspaceSymbol` or `SymbolInformation` JSON and normalizes it internally.
- Depending on Rust Analyzer capabilities, this command may return an LSP error instead of a resolved symbol.

### `incoming-calls`

Find incoming call hierarchy edges for the symbol at a position.

```bash
ra-lsp incoming-calls \
  --relative-path src/main.rs \
  --line 34 \
  --character 5
```

Current success payload:

- `{ "result": CallHierarchyIncomingCall[] }`

### `outgoing-calls`

Find outgoing call hierarchy edges for the symbol at a position.

```bash
ra-lsp outgoing-calls \
  --relative-path src/main.rs \
  --line 34 \
  --character 5
```

Current success payload:

- `{ "result": CallHierarchyOutgoingCall[] }`

### `incoming-calls-recursive`

Traverse incoming calls recursively.

```bash
ra-lsp incoming-calls-recursive \
  --relative-path src/main.rs \
  --line 24 \
  --character 5 \
  --max-depth 10
```

Current success payload:

- `{ "result": [[CallHierarchyItem, CallHierarchyIncomingCall[]], ...] }`

Notes:

- `--max-depth` is optional.
- Omitting `--max-depth` leaves recursion unbounded at the CLI layer.

### `outgoing-calls-recursive`

Traverse outgoing calls recursively.

```bash
ra-lsp outgoing-calls-recursive \
  --relative-path src/main.rs \
  --line 39 \
  --character 5 \
  --max-depth 10
```

Current success payload:

- `{ "result": [[CallHierarchyItem, CallHierarchyOutgoingCall[]], ...] }`

### `analyze-func-deps-graph`

Analyze dependency edges among an explicit target set.

At least one target is required. Targets can be:

- regular function targets via repeated `--function-target <PATH,LINE,CHARACTER>`
- trait implementation targets via repeated `--trait-function-target <TARGET_DIR,TRAIT_NAME>`

Optional metadata can be attached by index:

- `--function-target-extra <JSON>`
- `--trait-function-target-extra <JSON>`

Rules:

- Extra metadata must be a JSON object.
- If any `--function-target-extra` values are provided, their count must match the number of `--function-target` values.
- If any `--trait-function-target-extra` values are provided, their count must match the number of `--trait-function-target` values.
- Function target paths may be relative paths, absolute paths, or `file://...` URIs.
- Trait target directories may be relative paths, absolute paths, or `file://...` URIs.

Trait-only example:

```bash
ra-lsp analyze-func-deps-graph \
  --trait-function-target src,Chain
```

Function-only example:

```bash
ra-lsp analyze-func-deps-graph \
  --function-target src/main.rs,34,0 \
  --function-target src/main.rs,24,0
```

Mixed targets with metadata:

```bash
ra-lsp analyze-func-deps-graph \
  --trait-function-target src,Chain \
  --trait-function-target-extra '{"label":"core"}' \
  --function-target src/main.rs,34,0 \
  --function-target-extra '{"ticket":"ABC-123"}'
```

Current success payload:

- `{ "result": AnalyzeFuncDepsGraphItem[] }`

Observed result item shape from the current implementation:

```json
{
  "result": [
    {
      "fn_type": "trait_impl",
      "function_name": "a",
      "file_uri": "file:///workspace/src/main.rs",
      "range": {
        "start": { "line": 50, "character": 4 },
        "end": { "line": 52, "character": 5 }
      },
      "extra": {
        "trait_name": "Chain",
        "label": "core"
      },
      "dependencies": [
        {
          "fn_type": "trait_impl",
          "file_uri": "file:///workspace/src/main.rs",
          "function_name": "b",
          "range": {
            "start": { "line": 54, "character": 4 },
            "end": { "line": 56, "character": 5 }
          }
        }
      ]
    }
  ]
}
```

Behavior notes:

- If a trait is not found, the result is an empty array.
- If a target directory exists but contains no matching implementations, the result is an empty array.
- Regular function targets that do not resolve to a function or method surface an LSP error.
- Mixed trait and regular-function target sets are supported.

## AI-Agent Usage Notes

For automation, the most stable conventions to rely on are:

- Always parse `stdout` as a single JSON envelope.
- Check `error` before consuming `result`.
- Treat all positions as zero-based.
- Prefer `--relative-path` only when you control the current working directory.
- Prefer `--uri` when your caller already has canonical `file://` paths.
- Pass explicit `--workspace` and `--initialize-params` in long-lived automation to avoid cwd-dependent behavior.
- Expect daemon reuse across sequential commands in the same workspace.
- Use `status` and `stop` for lifecycle observability and cleanup.

## Development

Build:

```bash
cargo build -p multilspy-cli
```

Run:

```bash
cargo run -p multilspy-cli -- --help
```

Test:

```bash
cargo test -p multilspy-cli
```

Format:

```bash
cargo fmt
```

Lint:

```bash
cargo clippy -p multilspy-cli --all-targets
```
