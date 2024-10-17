pub mod normalize;
pub mod quantify;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    /// Normalize feature counts.
    Normalize(normalize::Args),
    /// Gene expression quantification.
    Quantify(quantify::Args),
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
