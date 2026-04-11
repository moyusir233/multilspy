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
