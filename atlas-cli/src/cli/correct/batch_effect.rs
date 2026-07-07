use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Metadata source.
    #[arg(long, required = true)]
    pub metadata: PathBuf,

    /// Input source.
    pub src: PathBuf,
}
