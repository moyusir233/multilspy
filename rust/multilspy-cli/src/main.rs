use clap::Parser;
use commands::server::ServerCommand;

mod commands;
mod daemon;
mod error;

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
        max_depth: u32,
    },
    /// Get recursive outgoing calls from a function
    OutgoingCallsRecursive {
        file: String,
        line: u32,
        column: u32,
        #[arg(long, default_value_t = 10, help = "Maximum recursion depth (default: 10)")]
        max_depth: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("CLI: {:?}", cli);
    Ok(())
}
