mod create;

use self::create::create;
use crate::cli::dataset::Command;

pub async fn dataset(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Create(config) => create(config).await,
    }
}
