use sqlx::postgres::PgPoolOptions;

use crate::{cli::ServerConfig, server};

pub async fn run(config: ServerConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    server::serve(&config, pool).await?;

    Ok(())
}
