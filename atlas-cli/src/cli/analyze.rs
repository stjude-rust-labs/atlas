pub mod differential_expression;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    /// Differential expression analysis.
    DifferentialExpression(differential_expression::Args),
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}
