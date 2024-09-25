pub mod normalize;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    /// Normalize feature counts.
    Normalize(normalize::Args),
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
