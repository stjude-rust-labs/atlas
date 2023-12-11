use std::collections::HashSet;

use sqlx::postgres::PgPoolOptions;
use tokio::{fs::File, io::BufReader};
use tracing::info;

use crate::cli::ImportConfig;

pub async fn import(config: ImportConfig) -> anyhow::Result<()> {
    use crate::{
        counts::reader::read_counts,
        store::{
            annotations::find_or_create_annotations,
            configuration::find_or_create_configuration,
            count::create_counts,
            feature::{create_features, find_features},
            run::{create_run, run_exists},
            sample::find_or_create_sample,
        },
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
        config.strand_specification,
    )
    .await?;

    info!(id = configuration.id, "loaded configuration");

    let sample = find_or_create_sample(&mut tx, &config.sample_name).await?;

    info!(id = sample.id, "loaded sample");

    if run_exists(&mut tx, configuration.id, sample.id).await? {
        tx.rollback().await?;
        anyhow::bail!("run already exists for the sample and configuration");
    }

    let mut features = find_features(&mut tx, configuration.id).await?;

    info!("loaded {} feature", features.len());

    let mut reader = File::open(&config.src).await.map(BufReader::new)?;
    let counts = read_counts(
        &mut reader,
        config.format,
        &config.feature_name,
        config.strand_specification,
    )
    .await?;

    if features.is_empty() {
        let mut names = HashSet::new();
        names.extend(counts.keys().cloned());
        features = create_features(&mut tx, configuration.id, &names).await?;
        info!("created {} features", features.len());
    }

    let run = create_run(&mut tx, configuration.id, sample.id, &config.data_type).await?;
    create_counts(&mut tx, run.id, &features, &counts).await?;

    tx.commit().await?;

    Ok(())
}
