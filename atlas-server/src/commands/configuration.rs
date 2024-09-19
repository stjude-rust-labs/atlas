mod import;

use self::import::import;
use crate::cli::configuration::Command;

pub async fn configuration(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Import(config) => import(config).await,
    }
}
