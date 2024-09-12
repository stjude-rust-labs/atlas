use sqlx::postgres::PgPoolOptions;

use crate::{cli::dataset::AddConfig, store::dataset};

pub(super) async fn add(config: AddConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let dataset_id = config.dataset_id;
    let run_ids = &config.ids;

    let mut tx = pool.begin().await?;

    for run_id in run_ids {
        if let Err(e) = dataset::add(&mut *tx, dataset_id, *run_id).await {
            tx.rollback().await?;
            return Err(e.into());
        }
    }

    tx.commit().await?;

    Ok(())
}
