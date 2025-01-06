use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Perplexity of the conditional distribution.
    #[arg(long, default_value_t = 30.0)]
    pub perplexity: f64,

    /// Barnes-Hut angular size (θ).
    #[arg(long, default_value_t = 0.5)]
    pub theta: f64,

    /// Input sources.
    pub srcs: Vec<PathBuf>,
}
