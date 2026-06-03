use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Input source.
    pub src: PathBuf,
}
