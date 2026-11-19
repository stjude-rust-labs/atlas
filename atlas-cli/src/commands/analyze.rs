mod differential_expression;

use crate::cli::{self, analyze::Command};

pub fn analyze(args: cli::analyze::Args) -> anyhow::Result<()> {
    match args.command {
        Command::DifferentialExpression(args) => differential_expression::run(args),
    }
}
