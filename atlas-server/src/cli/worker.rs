use clap::Parser;

#[derive(Debug, Parser)]
pub struct WorkerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,
}
