mod cli;
mod commands;

use clap::Parser;

use self::{
    cli::{Cli, Command},
    commands::normalize,
};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Normalize(args) => normalize(args)?,
    }

    Ok(())
}
