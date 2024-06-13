pub mod configuration;
pub mod run;

use std::net::SocketAddr;

use clap::{Parser, Subcommand};

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
pub struct ServerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The socket address the server binds to.
    #[clap(long, env = "BIND_ADDRESS", default_value = "127.0.0.1:3000")]
    pub bind: SocketAddr,
}

#[derive(Debug, Parser)]
pub struct WorkerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,
}
