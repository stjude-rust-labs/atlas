pub mod batch_effect;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    /// Correct batch effects.
    BatchEffect(batch_effect::Args),
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}
