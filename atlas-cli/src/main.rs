mod cli;
mod commands;

use std::io;

use clap::Parser;

use self::{
    cli::{Cli, Command},
    commands::{normalize, quantify},
};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let cli = Cli::parse();

    match cli.command {
        Command::Normalize(args) => normalize(args)?,
        Command::Quantify(args) => quantify(args)?,
    }

    Ok(())
}
