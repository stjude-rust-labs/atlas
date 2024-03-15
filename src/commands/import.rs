use std::{collections::HashSet, io, path::Path};

use sqlx::{postgres::PgPoolOptions, Postgres, Transaction};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tracing::info;

use crate::{cli::ImportConfig, counts::Format, store::StrandSpecification};

pub async fn import(config: ImportConfig) -> anyhow::Result<()> {
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
        config.strand_specification,
    )
    .await?;

    info!(id = configuration.id, "loaded configuration");

    let result = if config.sample_sheet {
        import_many(
            &mut tx,
            &config.src,
            configuration.id,
            config.format,
            &config.feature_name,
            config.strand_specification,
            &config.data_type,
        )
        .await
    } else {
        import_one(
            &mut tx,
            &config.src,
            configuration.id,
            &config.sample_name,
            config.format,
            &config.feature_name,
            config.strand_specification,
            &config.data_type,
        )
        .await
    };

    match result {
        Ok(()) => tx.commit().await?,
        Err(e) => {
            tx.rollback().await?;
            anyhow::bail!("{e}");
        }
    }

    Ok(())
}

async fn import_one<P>(
    tx: &mut Transaction<'_, Postgres>,
    src: P,
    configuration_id: i32,
    sample_name: &str,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
    data_type: &str,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    use crate::{
        counts::reader::read_counts,
        store::{
            count::create_counts,
            feature::{create_features, find_features},
            run::{create_run, run_exists},
            sample::find_or_create_sample,
        },
    };

    let sample = find_or_create_sample(tx, sample_name).await?;

    info!(id = sample.id, "loaded sample");

    if run_exists(tx, configuration_id, sample.id).await? {
        anyhow::bail!("run already exists for the sample and configuration");
    }

    let mut features = find_features(tx, configuration_id).await?;

    info!("loaded {} feature", features.len());

    let mut reader = File::open(src).await.map(BufReader::new)?;
    let counts = read_counts(&mut reader, format, feature_name, strand_specification).await?;

    if features.is_empty() {
        let mut names = HashSet::new();
        names.extend(counts.keys().cloned());
        features = create_features(tx, configuration_id, &names).await?;
        info!("created {} features", features.len());
    }

    let run = create_run(tx, configuration_id, sample.id, data_type).await?;
    create_counts(tx, run.id, &features, &counts).await?;

    Ok(())
}

async fn import_many<P>(
    tx: &mut Transaction<'_, Postgres>,
    sample_sheet_src: P,
    configuration_id: i32,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
    data_type: &str,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    const DELIMITER: char = '\t';

    let f = File::open(sample_sheet_src).await.map(BufReader::new)?;

    let mut lines = f.lines();

    while let Some(line) = lines.next_line().await? {
        let (sample_name, src) = line
            .split_once(DELIMITER)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;

        import_one(
            tx,
            src,
            configuration_id,
            sample_name,
            format,
            feature_name,
            strand_specification,
            data_type,
        )
        .await?;
    }

    Ok(())
}
