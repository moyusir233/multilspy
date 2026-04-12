use clap::Parser;
use commands::server::ServerCommand;

mod commands;
mod daemon;
mod error;
mod ipc;
mod position_utils;

use error::CliError;

type Result<T> = std::result::Result<T, CliError>;

#[derive(Parser, Debug)]
#[command(name = "multilspy", version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(long, short, help = "Path to the project root (default: current working directory)")]
    pub project: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Manage the multilspy server
    Server {
        #[command(subcommand)]
        command: ServerCommand,
    },
    /// Internal: Run the daemon (not for user use)
    Daemon,
    /// Get definition of a symbol
    Definition {
        file: String,
        line: u32,
        column: u32,
    },
    /// Get type definition of a symbol
    TypeDefinition {
        file: String,
        line: u32,
        column: u32,
    },
    /// Get all references to a symbol
    References {
        file: String,
        line: u32,
        column: u32,
    },
    /// Get all symbols in a document
    DocumentSymbols {
        file: String,
    },
    /// Get implementations of a function or trait
    Implementation {
        file: String,
        line: u32,
        column: u32,
    },
    /// Get all callers of a function
    IncomingCalls {
        file: String,
        line: u32,
        column: u32,
    },
    /// Get all functions called by a function
    OutgoingCalls {
        file: String,
        line: u32,
        column: u32,
    },
    /// Get recursive incoming calls to a function
    IncomingCallsRecursive {
        file: String,
        line: u32,
        column: u32,
        #[arg(long, default_value_t = 10, help = "Maximum recursion depth (default: 10)")]
        max_depth: usize,
    },
    /// Get recursive outgoing calls from a function
    OutgoingCallsRecursive {
        file: String,
        line: u32,
        column: u32,
        #[arg(long, default_value_t = 10, help = "Maximum recursion depth (default: 10)")]
        max_depth: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server { command } => {
            commands::server::handle(command).await?;
        }
        Commands::Daemon => {
            daemon::manager::run_daemon().await?;
        }
        Commands::Definition { file, line, column } => {
            commands::definition::handle(cli.project.as_deref(), file, *line, *column).await?;
        }
        Commands::TypeDefinition { file, line, column } => {
            commands::type_definition::handle(cli.project.as_deref(), file, *line, *column).await?;
        }
        Commands::References { file, line, column } => {
            commands::references::handle(cli.project.as_deref(), file, *line, *column).await?;
        }
        Commands::DocumentSymbols { file } => {
            commands::document_symbols::handle(cli.project.as_deref(), file).await?;
        }
        Commands::Implementation { file, line, column } => {
            commands::implementation::handle(cli.project.as_deref(), file, *line, *column).await?;
        }
        Commands::IncomingCalls { file, line, column } => {
            commands::incoming_calls::handle(cli.project.as_deref(), file, *line, *column).await?;
        }
        Commands::OutgoingCalls { file, line, column } => {
            commands::outgoing_calls::handle(cli.project.as_deref(), file, *line, *column).await?;
        }
        Commands::IncomingCallsRecursive { file, line, column, max_depth } => {
            commands::incoming_calls_recursive::handle(cli.project.as_deref(), file, *line, *column, Some(*max_depth)).await?;
        }
        Commands::OutgoingCallsRecursive { file, line, column, max_depth } => {
            commands::outgoing_calls_recursive::handle(cli.project.as_deref(), file, *line, *column, Some(*max_depth)).await?;
        }
    }

    Ok(())
}
