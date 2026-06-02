mod tsne;

use crate::cli::{self, transform::Command};

pub fn transform(args: cli::transform::Args) -> anyhow::Result<()> {
    match args.command {
        Command::Tsne(args) => tsne::run(args),
    }
}
