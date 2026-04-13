mod daemon;
mod ipc;
mod lifecycle;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use ipc::{IpcRequest, IpcResponse};

#[derive(Parser)]
#[command(
    name = "multilspy",
    about = "LSP CLI for AI agents — persistent daemon avoids repeated server initialization",
    version
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

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Go to definition at a given position")]
    Definition {
        #[arg(long, help = "Document URI (file:///...)")]
        uri: String,
        #[arg(long, help = "Zero-based line number")]
        line: u32,
        #[arg(long, help = "Zero-based character offset")]
        character: u32,
    },

    #[command(about = "Go to type definition at a given position")]
    TypeDefinition {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(about = "Go to implementation at a given position")]
    Implementation {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(about = "Find all references at a given position")]
    References {
        #[arg(long)]
        uri: String,
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

    #[command(about = "List document symbols")]
    DocumentSymbols {
        #[arg(long, help = "Document URI (file:///...)")]
        uri: String,
    },

    #[command(about = "Find incoming calls at a given position")]
    IncomingCalls {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(about = "Find outgoing calls at a given position")]
    OutgoingCalls {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
    },

    #[command(about = "Find incoming calls recursively at a given position")]
    IncomingCallsRecursive {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
        #[arg(long, help = "Maximum recursion depth")]
        max_depth: Option<usize>,
    },

    #[command(about = "Find outgoing calls recursively at a given position")]
    OutgoingCallsRecursive {
        #[arg(long)]
        uri: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        character: u32,
        #[arg(long, help = "Maximum recursion depth")]
        max_depth: Option<usize>,
    },

    #[command(about = "Show daemon status for the current workspace")]
    Status,

    #[command(about = "Stop the daemon for the current workspace")]
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
        } => {
            if let Err(e) = daemon::run_daemon(workspace, initialize_params).await {
                eprintln!(
                    "{}",
                    serde_json::json!({"error": {"code": -1, "message": e.to_string()}})
                );
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        cmd => run_client_command(cmd, cli.workspace, cli.initialize_params).await,
    }
}

async fn run_client_command(
    cmd: Command,
    workspace_arg: Option<PathBuf>,
    init_params_arg: Option<PathBuf>,
) -> ExitCode {
    let workspace = workspace_arg
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let initialize_params = init_params_arg
        .unwrap_or_else(|| workspace.join("ra_initialize_params.json"));

    let port = match lifecycle::ensure_daemon(&workspace, &initialize_params).await {
        Ok(p) => p,
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
            uri,
            line,
            character,
        } => (
            "definition",
            serde_json::to_value(ipc::PositionParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
            })?,
        ),

        Command::TypeDefinition {
            uri,
            line,
            character,
        } => (
            "type-definition",
            serde_json::to_value(ipc::PositionParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
            })?,
        ),

        Command::Implementation {
            uri,
            line,
            character,
        } => (
            "implementation",
            serde_json::to_value(ipc::PositionParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
            })?,
        ),

        Command::References {
            uri,
            line,
            character,
            include_declaration,
        } => (
            "references",
            serde_json::to_value(ipc::ReferencesIpcParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
                include_declaration: *include_declaration,
            })?,
        ),

        Command::DocumentSymbols { uri } => (
            "document-symbols",
            serde_json::to_value(ipc::DocumentSymbolsIpcParams { uri: uri.clone() })?,
        ),

        Command::IncomingCalls {
            uri,
            line,
            character,
        } => (
            "incoming-calls",
            serde_json::to_value(ipc::PositionParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
            })?,
        ),

        Command::OutgoingCalls {
            uri,
            line,
            character,
        } => (
            "outgoing-calls",
            serde_json::to_value(ipc::PositionParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
            })?,
        ),

        Command::IncomingCallsRecursive {
            uri,
            line,
            character,
            max_depth,
        } => (
            "incoming-calls-recursive",
            serde_json::to_value(ipc::RecursiveCallsIpcParams {
                uri: uri.clone(),
                line: *line,
                character: *character,
                max_depth: *max_depth,
            })?,
        ),

        Command::OutgoingCallsRecursive {
            uri,
            line,
            character,
            max_depth,
        } => (
            "outgoing-calls-recursive",
            serde_json::to_value(ipc::RecursiveCallsIpcParams {
                uri: uri.clone(),
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
