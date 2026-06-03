pub mod tsne;
pub mod vst;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    /// Dimension reduction using t-distributed Stochastic Neighbor Embedding (t-SNE).
    Tsne(tsne::Args),
    /// Variance stabilizing transformation (VST).
    Vst(vst::Args),
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}
