use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Metadata source.
    #[arg(long, required = true)]
    pub metadata: PathBuf,

    /// Reference group name.
    #[arg(long)]
    pub reference: Option<String>,

    /// Target group name.
    #[arg(long)]
    pub target: Option<String>,

    /// Input source.
    pub src: PathBuf,
}
