mod daemon;
mod ipc;
mod lifecycle;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Args, Parser, Subcommand};
use ipc::{IpcRequest, IpcResponse};
use multilspy_protocol::protocol::requests::InitializeParams;
use reqwest::Url;

const INIT_PARAMS_ENV_VAR: &str = "RA_LSP_INIT_PARAMS_PATH";

const ROOT_HELP: &str = r#"Configuration:
  - `--initialize-params <PATH>` points to an initialize params JSON file.
  - `RA_LSP_INIT_PARAMS_PATH` can also provide a full initialize params JSON file.
  - If both are set, the CLI merges them and `--initialize-params` overrides matching JSON fields.
  - If neither is set, the CLI falls back to `<workspace>/ra_initialize_params.json`.

File URI Input:
  - Commands that read a document accept either `--uri <file://...>` or `--relative-path <PATH>`.
  - `--relative-path` resolves against the current working directory and converts the path to a valid escaped `file://` URI.

JSON Output:
  - Every command prints a single JSON object.
  - Success shape: `{ "result": <command-specific JSON> }`
  - Error shape: `{ "error": { "code": -32602, "message": "..." } }`
"#;

const DEFINITION_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": Location[] }`
  - Empty matches return `{ "result": [] }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 24, "character": 4 }, "end": { "line": 24, "character": 16 } } }] }`
"#;

const TYPE_DEFINITION_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": Location[] }`
  - Empty matches return `{ "result": [] }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 4, "character": 0 }, "end": { "line": 6, "character": 1 } } }] }`
"#;

const IMPLEMENTATION_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": Location[] }`
  - Empty matches return `{ "result": [] }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 8, "character": 0 }, "end": { "line": 12, "character": 1 } } }] }`
"#;

const REFERENCES_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.
  - `--include-declaration` controls whether the declaration location is included in `result`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": Location[] }`
  - Output may differ based on `--include-declaration`.
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 24, "character": 4 }, "end": { "line": 24, "character": 16 } } }, { "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 35, "character": 11 }, "end": { "line": 35, "character": 23 } } }] }`
"#;

const DOCUMENT_SYMBOLS_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": DocumentSymbol[] }`
  - Symbols can include nested `children`.
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "name": "Greeter", "kind": 11, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 2, "character": 1 } }, "selectionRange": { "start": { "line": 0, "character": 6 }, "end": { "line": 0, "character": 13 } }, "children": [] }] }`
"#;

const INCOMING_CALLS_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": CallHierarchyIncomingCall[] }`
  - Empty matches return `{ "result": [] }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "from": { "name": "main", "kind": 12, "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 39, "character": 0 }, "end": { "line": 42, "character": 1 } }, "selectionRange": { "start": { "line": 39, "character": 3 }, "end": { "line": 39, "character": 7 } } }, "fromRanges": [{ "start": { "line": 40, "character": 4 }, "end": { "line": 40, "character": 10 } }] }] }`
"#;

const OUTGOING_CALLS_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": CallHierarchyOutgoingCall[] }`
  - Empty matches return `{ "result": [] }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [{ "to": { "name": "create_hello", "kind": 12, "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 24, "character": 0 }, "end": { "line": 27, "character": 1 } }, "selectionRange": { "start": { "line": 24, "character": 3 }, "end": { "line": 24, "character": 15 } } }, "fromRanges": [{ "start": { "line": 35, "character": 11 }, "end": { "line": 35, "character": 23 } }] }] }`
"#;

const INCOMING_CALLS_RECURSIVE_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.
  - `--max-depth` limits how many call edges are traversed.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": [[CallHierarchyItem, CallHierarchyIncomingCall[]], ...] }`
  - Output varies when `--max-depth` is set.
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [[{ "name": "create_hello", "kind": 12, "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 24, "character": 0 }, "end": { "line": 27, "character": 1 } }, "selectionRange": { "start": { "line": 24, "character": 3 }, "end": { "line": 24, "character": 15 } } }, [{ "from": { "name": "helper", "kind": 12, "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 34, "character": 0 }, "end": { "line": 37, "character": 1 } }, "selectionRange": { "start": { "line": 34, "character": 3 }, "end": { "line": 34, "character": 9 } } }, "fromRanges": [{ "start": { "line": 35, "character": 11 }, "end": { "line": 35, "character": 23 } }] }]]] }`
"#;

const OUTGOING_CALLS_RECURSIVE_HELP: &str = r#"Input:
  - Use exactly one of `--uri <file://...>` or `--relative-path <PATH>`.
  - `--max-depth` limits how many call edges are traversed.

Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

JSON Output:
  - Success schema: `{ "result": [[CallHierarchyItem, CallHierarchyOutgoingCall[]], ...] }`
  - Output varies when `--max-depth` is set.
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": [[{ "name": "main", "kind": 12, "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 39, "character": 0 }, "end": { "line": 42, "character": 1 } }, "selectionRange": { "start": { "line": 39, "character": 3 }, "end": { "line": 39, "character": 7 } } }, [{ "to": { "name": "helper", "kind": 12, "uri": "file:///workspace/src/main.rs", "range": { "start": { "line": 34, "character": 0 }, "end": { "line": 37, "character": 1 } }, "selectionRange": { "start": { "line": 34, "character": 3 }, "end": { "line": 34, "character": 9 } } }, "fromRanges": [{ "start": { "line": 40, "character": 4 }, "end": { "line": 40, "character": 10 } }] }]]] }`
"#;

const STATUS_HELP: &str = r#"Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

File URI Input:
  - `--relative-path` is not applicable to `status`.

JSON Output:
  - Success schema: `{ "result": { "workspace": string, "pid": number, "port": number, "uptime_secs": number } }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": { "workspace": "/workspace/project", "pid": 4242, "port": 53741, "uptime_secs": 18 } }`
"#;

const STOP_HELP: &str = r#"Initialize Params:
  - `RA_LSP_INIT_PARAMS_PATH` provides a base initialize params JSON file.
  - `--initialize-params` overrides matching fields from that JSON file.

File URI Input:
  - `--relative-path` is not applicable to `stop`.

JSON Output:
  - Success schema: `{ "result": "shutdown_ack" }`
  - Error schema: `{ "error": { "code": number, "message": string } }`
  - Example:
    `{ "result": "shutdown_ack" }`
"#;

#[derive(Debug, Clone, Args)]
struct FileUriArgs {
    #[arg(
        long,
        conflicts_with = "relative_path",
        required_unless_present = "relative_path",
        help = "Document URI (file:///...)"
    )]
    uri: Option<String>,

    #[arg(
        long,
        short = 'p',
        value_name = "PATH",
        conflicts_with = "uri",
        required_unless_present = "uri",
        help = "Path resolved against the current working directory and converted to file://"
    )]
    relative_path: Option<PathBuf>,
}

impl FileUriArgs {
    fn resolve_uri(&self) -> anyhow::Result<String> {
        match (&self.uri, &self.relative_path) {
            (Some(uri), None) => Ok(uri.clone()),
            (None, Some(relative_path)) => resolve_relative_path_to_file_uri(relative_path),
            (Some(_), Some(_)) => anyhow::bail!(
                "cannot use --uri together with --relative-path; pass only one file input"
            ),
            (None, None) => {
                anyhow::bail!("one of --uri or --relative-path must be provided")
            }
        }
    }
}

#[derive(Debug)]
struct ResolvedInitializeParams {
    path: PathBuf,
    temporary_path: Option<PathBuf>,
}

impl ResolvedInitializeParams {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ResolvedInitializeParams {
    fn drop(&mut self) {
        if let Some(path) = &self.temporary_path {
            let _ = fs::remove_file(path);
        }
    }
}

#[derive(Parser)]
#[command(
    name = "multilspy",
    about = "LSP CLI for AI agents — persistent daemon avoids repeated server initialization",
    version,
    after_help = ROOT_HELP
)]
struct Cli {
    #[arg(
        long,
        short = 'w',
        global = true,
        help = "Workspace root directory (defaults to current directory)"
    )]
    workspace: Option<PathBuf>,

    #[arg(
        long,
        short = 'i',
        global = true,
        help = "Path to ra_initialize_params.json"
    )]
    initialize_params: Option<PathBuf>,

    #[arg(
        short = 't',
        long = "wait-work-done-progress-create-max-time",
        global = true,
        help = "Max wait time (seconds) for rust-analyzer to create workDoneProgress"
    )]
    wait_work_done_progress_create_max_time_secs: Option<u64>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Go to definition at a given position", after_help = DEFINITION_HELP)]
    Definition {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long, help = "Zero-based line number")]
        line: u32,
        #[arg(long, help = "Zero-based character offset")]
        character: u32,
    },

    #[command(
        about = "Go to type definition at a given position",
        after_help = TYPE_DEFINITION_HELP
    )]
    TypeDefinition {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(
        about = "Go to implementation at a given position",
        after_help = IMPLEMENTATION_HELP
    )]
    Implementation {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(about = "Find all references at a given position", after_help = REFERENCES_HELP)]
    References {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
        #[arg(
            long,
            default_value_t = true,
            default_missing_value = "true",
            num_args = 0..=1,
            require_equals = false,
            help = "Include the declaration itself"
        )]
        include_declaration: bool,
    },

    #[command(about = "List document symbols", after_help = DOCUMENT_SYMBOLS_HELP)]
    DocumentSymbols {
        #[command(flatten)]
        file: FileUriArgs,
    },

    #[command(
        about = "Find incoming calls at a given position",
        after_help = INCOMING_CALLS_HELP
    )]
    IncomingCalls {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(
        about = "Find outgoing calls at a given position",
        after_help = OUTGOING_CALLS_HELP
    )]
    OutgoingCalls {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(
        about = "Find incoming calls recursively at a given position",
        after_help = INCOMING_CALLS_RECURSIVE_HELP
    )]
    IncomingCallsRecursive {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
        #[arg(long, help = "Maximum recursion depth")]
        max_depth: Option<usize>,
    },

    #[command(
        about = "Find outgoing calls recursively at a given position",
        after_help = OUTGOING_CALLS_RECURSIVE_HELP
    )]
    OutgoingCallsRecursive {
        #[command(flatten)]
        file: FileUriArgs,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
        #[arg(long, help = "Maximum recursion depth")]
        max_depth: Option<usize>,
    },

    #[command(
        about = "Show daemon status for the current workspace",
        after_help = STATUS_HELP
    )]
    Status,

    #[command(about = "Stop the daemon for the current workspace", after_help = STOP_HELP)]
    Stop,

    #[command(
        about = "Run the daemon process (internal — used by auto-spawn)",
        hide = true
    )]
    Daemon {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long = "initialize-params")]
        initialize_params: PathBuf,
        #[arg(long = "wait-work-done-progress-create-max-time")]
        wait_work_done_progress_create_max_time_secs: Option<u64>,
    },
}

fn output_json(resp: &IpcResponse) {
    let json = serde_json::to_string(resp).expect("failed to serialize IpcResponse");
    println!("{}", json);
}

fn output_error(code: i32, message: String) {
    let resp = IpcResponse::error(code, message);
    output_json(&resp);
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Daemon {
            workspace,
            initialize_params,
            wait_work_done_progress_create_max_time_secs,
        } => {
            if let Err(e) = daemon::run_daemon(
                workspace,
                initialize_params,
                wait_work_done_progress_create_max_time_secs,
            )
            .await
            {
                eprintln!(
                    "{}",
                    serde_json::json!({"error": {"code": -1, "message": e.to_string()}})
                );
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        cmd => {
            run_client_command(
                cmd,
                cli.workspace,
                cli.initialize_params,
                cli.wait_work_done_progress_create_max_time_secs,
            )
            .await
        }
    }
}

async fn run_client_command(
    cmd: Command,
    workspace_arg: Option<PathBuf>,
    init_params_arg: Option<PathBuf>,
    wait_work_done_progress_create_max_time_secs: Option<u64>,
) -> ExitCode {
    let workspace = workspace_arg
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let initialize_params = match resolve_initialize_params_path(&workspace, init_params_arg) {
        Ok(path) => path,
        Err(e) => {
            output_error(ipc::ERR_INVALID_PARAMS, e.to_string());
            return ExitCode::FAILURE;
        }
    };

    let port = match lifecycle::ensure_daemon(
        &workspace,
        initialize_params.path(),
        wait_work_done_progress_create_max_time_secs,
        matches!(&cmd, Command::Stop),
    )
    .await
    {
        Ok(p) => {
            if p == 0 && matches!(&cmd, Command::Stop) {
                return ExitCode::SUCCESS;
            }
            p
        }
        Err(e) => {
            output_error(-1, format!("failed to connect to daemon: {}", e));
            return ExitCode::FAILURE;
        }
    };

    let request = match build_request(&cmd) {
        Ok(r) => r,
        Err(e) => {
            output_error(ipc::ERR_INVALID_PARAMS, e.to_string());
            return ExitCode::FAILURE;
        }
    };

    match ipc::send_request(port, &request).await {
        Ok(resp) => {
            output_json(&resp);
            if resp.error.is_some() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            output_error(ipc::ERR_INTERNAL, format!("IPC request failed: {}", e));
            ExitCode::FAILURE
        }
    }
}

fn build_request(cmd: &Command) -> anyhow::Result<IpcRequest> {
    let (method, params) = match cmd {
        Command::Definition {
            file,
            line,
            character,
        } => (
            "definition",
            serde_json::to_value(ipc::PositionParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
            })?,
        ),

        Command::TypeDefinition {
            file,
            line,
            character,
        } => (
            "type-definition",
            serde_json::to_value(ipc::PositionParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
            })?,
        ),

        Command::Implementation {
            file,
            line,
            character,
        } => (
            "implementation",
            serde_json::to_value(ipc::PositionParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
            })?,
        ),

        Command::References {
            file,
            line,
            character,
            include_declaration,
        } => (
            "references",
            serde_json::to_value(ipc::ReferencesIpcParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
                include_declaration: *include_declaration,
            })?,
        ),

        Command::DocumentSymbols { file } => (
            "document-symbols",
            serde_json::to_value(ipc::DocumentSymbolsIpcParams {
                uri: file.resolve_uri()?,
            })?,
        ),

        Command::IncomingCalls {
            file,
            line,
            character,
        } => (
            "incoming-calls",
            serde_json::to_value(ipc::PositionParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
            })?,
        ),

        Command::OutgoingCalls {
            file,
            line,
            character,
        } => (
            "outgoing-calls",
            serde_json::to_value(ipc::PositionParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
            })?,
        ),

        Command::IncomingCallsRecursive {
            file,
            line,
            character,
            max_depth,
        } => (
            "incoming-calls-recursive",
            serde_json::to_value(ipc::RecursiveCallsIpcParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
                max_depth: *max_depth,
            })?,
        ),

        Command::OutgoingCallsRecursive {
            file,
            line,
            character,
            max_depth,
        } => (
            "outgoing-calls-recursive",
            serde_json::to_value(ipc::RecursiveCallsIpcParams {
                uri: file.resolve_uri()?,
                line: *line,
                character: *character,
                max_depth: *max_depth,
            })?,
        ),

        Command::Status => ("status", serde_json::json!(null)),

        Command::Stop => ("shutdown", serde_json::json!(null)),

        Command::Daemon { .. } => unreachable!(),
    };

    Ok(IpcRequest {
        method: method.to_string(),
        params,
    })
}

fn resolve_relative_path_to_file_uri(input_path: &Path) -> anyhow::Result<String> {
    let cwd = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("failed to resolve current working directory: {}", e))?;
    let resolved_path = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        cwd.join(input_path)
    };

    let canonical_path = resolved_path
        .canonicalize()
        .map_err(|e| format_path_resolution_error(input_path, &resolved_path, e))?;
    Url::from_file_path(&canonical_path)
        .map_err(|()| {
            anyhow::anyhow!(
                "failed to convert path '{}' to a valid file:// URI",
                canonical_path.display()
            )
        })
        .map(|url| url.to_string())
}

fn format_path_resolution_error(
    input_path: &Path,
    resolved_path: &Path,
    error: std::io::Error,
) -> anyhow::Error {
    let message = match error.kind() {
        ErrorKind::NotFound => format!(
            "relative path '{}' resolved to '{}' but that path does not exist",
            input_path.display(),
            resolved_path.display()
        ),
        ErrorKind::PermissionDenied => format!(
            "relative path '{}' resolved to '{}' but could not be accessed: permission denied",
            input_path.display(),
            resolved_path.display()
        ),
        _ => format!(
            "failed to resolve relative path '{}' (resolved as '{}'): {}",
            input_path.display(),
            resolved_path.display(),
            error
        ),
    };
    anyhow::anyhow!(message)
}

fn resolve_initialize_params_path(
    workspace: &Path,
    cli_path: Option<PathBuf>,
) -> anyhow::Result<ResolvedInitializeParams> {
    let env_path = std::env::var_os(INIT_PARAMS_ENV_VAR).map(PathBuf::from);
    let default_path = workspace.join("ra_initialize_params.json");

    match (env_path, cli_path) {
        (Some(env_path), Some(cli_path)) => {
            let env_json = load_initialize_params_value(
                &env_path,
                &format!("{}='{}'", INIT_PARAMS_ENV_VAR, env_path.display()),
            )?;
            let cli_json = load_initialize_params_value(
                &cli_path,
                &format!("--initialize-params '{}'", cli_path.display()),
            )?;

            let mut merged = env_json;
            merge_json_values(&mut merged, cli_json);
            validate_initialize_params_value(
                &merged,
                &format!(
                    "merged initialize params from {} and --initialize-params",
                    INIT_PARAMS_ENV_VAR
                ),
            )?;

            let temporary_path = write_merged_initialize_params_file(&merged)?;
            Ok(ResolvedInitializeParams {
                path: temporary_path.clone(),
                temporary_path: Some(temporary_path),
            })
        }
        (Some(env_path), None) => {
            validate_initialize_params_file(
                &env_path,
                &format!("{}='{}'", INIT_PARAMS_ENV_VAR, env_path.display()),
            )?;
            Ok(ResolvedInitializeParams {
                path: env_path,
                temporary_path: None,
            })
        }
        (None, Some(cli_path)) => {
            validate_initialize_params_file(
                &cli_path,
                &format!("--initialize-params '{}'", cli_path.display()),
            )?;
            Ok(ResolvedInitializeParams {
                path: cli_path,
                temporary_path: None,
            })
        }
        (None, None) => {
            validate_initialize_params_file(
                &default_path,
                &format!("default initialize params '{}'", default_path.display()),
            )?;
            Ok(ResolvedInitializeParams {
                path: default_path,
                temporary_path: None,
            })
        }
    }
}

fn validate_initialize_params_file(path: &Path, source: &str) -> anyhow::Result<()> {
    let value = load_initialize_params_value(path, source)?;
    validate_initialize_params_value(&value, source)
}

fn load_initialize_params_value(path: &Path, source: &str) -> anyhow::Result<serde_json::Value> {
    let _metadata = fs::metadata(path)
        .map_err(|error| format_initialize_params_io_error(path, source, "access", error))?;
    let content = fs::read_to_string(path)
        .map_err(|error| format_initialize_params_io_error(path, source, "read", error))?;
    serde_json::from_str(&content).map_err(|error| {
        anyhow::anyhow!(
            "invalid JSON in initialize params file from {} at '{}': {}",
            source,
            path.display(),
            error
        )
    })
}

fn validate_initialize_params_value(value: &serde_json::Value, source: &str) -> anyhow::Result<()> {
    serde_json::from_value::<InitializeParams>(value.clone()).map_err(|error| {
        anyhow::anyhow!(
            "invalid initialize params structure from {}: {}",
            source,
            error
        )
    })?;
    Ok(())
}

fn merge_json_values(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => merge_json_values(base_value, overlay_value),
                    None => {
                        base_map.insert(key, overlay_value);
                    }
                }
            }
        }
        (base_value, overlay_value) => *base_value = overlay_value,
    }
}

fn write_merged_initialize_params_file(value: &serde_json::Value) -> anyhow::Result<PathBuf> {
    let mut path = std::env::temp_dir();
    path.push("multilspy-cli");
    fs::create_dir_all(&path)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!(
        "merged-initialize-params-{}-{}.json",
        std::process::id(),
        timestamp
    ));

    let content = serde_json::to_vec_pretty(value)?;
    fs::write(&path, content)?;
    Ok(path)
}

fn format_initialize_params_io_error(
    path: &Path,
    source: &str,
    action: &str,
    error: std::io::Error,
) -> anyhow::Error {
    let detail = match error.kind() {
        ErrorKind::NotFound => "file does not exist".to_string(),
        ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => error.to_string(),
    };
    anyhow::anyhow!(
        "failed to {} initialize params file from {} at '{}': {}",
        action,
        source,
        path.display(),
        detail
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};

    fn process_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "multilspy-cli-test-{}-{}-{}",
            name,
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&dir).expect("failed to create temp test directory");
        dir
    }

    fn write_json_file(dir: &Path, name: &str, value: &serde_json::Value) -> PathBuf {
        let path = dir.join(name);
        fs::write(
            &path,
            serde_json::to_vec_pretty(value).expect("failed to serialize test json"),
        )
        .expect("failed to write test json");
        path
    }

    #[test]
    fn merge_json_values_recursively_overrides_cli_fields() {
        let mut base = json!({
            "capabilities": {
                "workspace": { "workspaceFolders": true },
                "window": { "workDoneProgress": false }
            },
            "trace": "messages"
        });
        let overlay = json!({
            "capabilities": {
                "window": { "workDoneProgress": true }
            },
            "initializationOptions": { "check": { "command": "clippy" } }
        });

        merge_json_values(&mut base, overlay);

        assert_eq!(
            base,
            json!({
                "capabilities": {
                    "workspace": { "workspaceFolders": true },
                    "window": { "workDoneProgress": true }
                },
                "trace": "messages",
                "initializationOptions": { "check": { "command": "clippy" } }
            })
        );
    }

    #[test]
    fn resolve_initialize_params_path_merges_env_and_cli_files() {
        let _guard = process_lock().lock().unwrap();
        let dir = unique_temp_dir("init-merge");
        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace).unwrap();

        let env_path = write_json_file(
            &dir,
            "env.json",
            &json!({
                "capabilities": {
                    "workspace": { "workspaceFolders": true }
                },
                "trace": "messages"
            }),
        );
        let cli_path = write_json_file(
            &dir,
            "cli.json",
            &json!({
                "capabilities": {
                    "window": { "workDoneProgress": true }
                },
                "trace": "verbose"
            }),
        );

        unsafe {
            std::env::set_var(INIT_PARAMS_ENV_VAR, &env_path);
        }
        let resolved = resolve_initialize_params_path(&workspace, Some(cli_path)).unwrap();
        let merged: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(resolved.path()).unwrap()).unwrap();

        assert_eq!(merged["trace"], "verbose");
        assert_eq!(
            merged["capabilities"]["workspace"]["workspaceFolders"],
            json!(true)
        );
        assert_eq!(
            merged["capabilities"]["window"]["workDoneProgress"],
            json!(true)
        );
        assert!(resolved.temporary_path.is_some());

        unsafe {
            std::env::remove_var(INIT_PARAMS_ENV_VAR);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolve_initialize_params_path_errors_for_missing_env_file() {
        let _guard = process_lock().lock().unwrap();
        let dir = unique_temp_dir("init-missing");
        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let missing = dir.join("missing.json");

        unsafe {
            std::env::set_var(INIT_PARAMS_ENV_VAR, &missing);
        }
        let error = resolve_initialize_params_path(&workspace, None).unwrap_err();
        assert!(error.to_string().contains("file does not exist"));
        assert!(error.to_string().contains(INIT_PARAMS_ENV_VAR));

        unsafe {
            std::env::remove_var(INIT_PARAMS_ENV_VAR);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolve_initialize_params_path_errors_for_invalid_structure() {
        let _guard = process_lock().lock().unwrap();
        let dir = unique_temp_dir("init-invalid");
        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let invalid_path = write_json_file(&dir, "invalid.json", &json!({ "trace": "verbose" }));

        unsafe {
            std::env::set_var(INIT_PARAMS_ENV_VAR, &invalid_path);
        }
        let error = resolve_initialize_params_path(&workspace, None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid initialize params structure")
        );

        unsafe {
            std::env::remove_var(INIT_PARAMS_ENV_VAR);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolve_relative_path_to_file_uri_escapes_special_characters() {
        let _guard = process_lock().lock().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        let dir = unique_temp_dir("relative-path");
        let file_path = dir.join("file with space #1.rs");
        fs::write(&file_path, "fn main() {}\n").unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let uri = resolve_relative_path_to_file_uri(Path::new("file with space #1.rs")).unwrap();
        assert!(uri.starts_with("file://"));
        assert!(uri.contains("file%20with%20space%20%231.rs"));

        std::env::set_current_dir(original_cwd).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolve_relative_path_to_file_uri_errors_for_missing_path() {
        let _guard = process_lock().lock().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        let dir = unique_temp_dir("relative-missing");
        std::env::set_current_dir(&dir).unwrap();

        let error = resolve_relative_path_to_file_uri(Path::new("missing.rs")).unwrap_err();
        assert!(error.to_string().contains("does not exist"));

        std::env::set_current_dir(original_cwd).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }
}
