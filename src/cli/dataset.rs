use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a dataset
    Create(CreateConfig),
}

#[derive(Debug, Parser)]
pub struct CreateConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The dataset name.
    pub name: String,
}
