use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod cli;
mod commands;
mod fs;

use std::io;

use clap::Parser;

use self::{
    cli::{Cli, Command},
    commands::{normalize, quantify, transform},
};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let cli = Cli::parse();

    match cli.command {
        Command::Normalize(args) => normalize(args)?,
        Command::Quantify(args) => quantify(args)?,
        Command::Transform(args) => transform(args)?,
    }

    Ok(())
}
