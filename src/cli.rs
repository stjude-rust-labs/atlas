pub mod configuration;
pub mod run;
mod server;

use clap::{Parser, Subcommand};

pub use self::server::ServerConfig;

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage configurations
    #[clap(subcommand)]
    Configuration(configuration::Command),
    /// Manage runs
    #[clap(subcommand)]
    Run(run::Command),
    /// Starts an atlas server and blocks indefinitely
    Server(ServerConfig),
    /// Starts an atlas worker.
    Worker(WorkerConfig),
}

#[derive(Debug, Parser)]
pub struct WorkerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,
}
