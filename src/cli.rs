use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Imports a sample into the database
    Import,
    /// Starts an altas server and blocks indefinitely
    Run(ServerConfig),
}

#[derive(Debug, Parser)]
pub struct ServerConfig {
    #[clap(long, env)]
    pub database_url: String,

    #[clap(long, env, default_value_t = 3000)]
    pub port: u16,
}
