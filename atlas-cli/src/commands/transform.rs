mod tsne;
mod vst;

use crate::cli::{self, transform::Command};

pub fn transform(args: cli::transform::Args) -> anyhow::Result<()> {
    match args.command {
        Command::Tsne(args) => tsne::run(args),
        Command::Vst(args) => vst::run(args),
    }
}
