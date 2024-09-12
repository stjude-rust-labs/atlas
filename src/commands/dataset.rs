mod add;
mod create;

use self::{add::add, create::create};
use crate::cli::dataset::Command;

pub async fn dataset(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Add(config) => add(config).await,
        Command::Create(config) => create(config).await,
    }
}
