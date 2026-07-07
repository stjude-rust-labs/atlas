mod batch_effect;

use crate::cli::{self, correct::Command};

pub fn correct(args: cli::correct::Args) -> anyhow::Result<()> {
    match args.command {
        Command::BatchEffect(args) => batch_effect::run(args),
    }
}
