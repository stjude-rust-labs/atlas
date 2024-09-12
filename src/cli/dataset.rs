use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Add a run to a dataset
    Add(AddConfig),
    /// Create a dataset
    Create(CreateConfig),
}

#[derive(Debug, Parser)]
pub struct AddConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The dataset ID.
    #[clap(long)]
    pub dataset_id: i32,

    /// Run IDs.
    pub ids: Vec<i32>,
}

#[derive(Debug, Parser)]
pub struct CreateConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The dataset name.
    pub name: String,
}
