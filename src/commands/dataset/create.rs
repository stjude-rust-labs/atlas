use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::{cli::dataset::CreateConfig, store::dataset};

pub(super) async fn create(config: CreateConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let dataset_id = dataset::create(&pool, &config.name).await?;

    info!(id = dataset_id, "created dataset");

    Ok(())
}
