mod import;

use self::import::import;
use crate::cli::run::Command;

pub async fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Import(config) => import(config).await,
    }
}
