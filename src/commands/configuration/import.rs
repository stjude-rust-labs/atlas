use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::cli::configuration::ImportConfig;

pub(super) async fn import(config: ImportConfig) -> anyhow::Result<()> {
    use crate::store::{
        annotations::find_or_create_annotations, configuration::find_or_create_configuration,
    };

    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let mut tx = pool.begin().await?;

    let annotations = find_or_create_annotations(
        &mut tx,
        &config.annotations_name,
        &config.annotations_genome_build,
    )
    .await?;

    info!(id = annotations.id, "loaded annotations");

    let configuration = find_or_create_configuration(
        &mut tx,
        annotations.id,
        &config.feature_type,
        &config.feature_name,
    )
    .await?;

    info!(id = configuration.id, "imported configuration");

    tx.commit().await?;

    Ok(())
}
