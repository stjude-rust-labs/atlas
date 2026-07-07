pub mod correct;
pub mod normalize;
pub mod quantify;
pub mod transform;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    /// Correct technical variations.
    Correct(correct::Args),
    /// Normalize feature counts.
    Normalize(normalize::Args),
    /// Transform feature counts.
    Transform(transform::Args),
    /// Gene expression quantification.
    Quantify(quantify::Args),
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
