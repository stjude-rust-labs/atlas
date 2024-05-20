use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
};

use sqlx::{postgres::PgPoolOptions, Postgres, Transaction};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tracing::info;

use crate::{
    cli::ImportConfig,
    counts::{feature_names_eq, reader::read_counts, Format},
    store::StrandSpecification,
};

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
    )
    .await?;

    info!(id = configuration.id, "loaded configuration");
    info!(src_count = config.srcs.len(), "reading srcs");

    let result = if config.sample_sheet {
        import_from_sample_sheets(
            &mut tx,
            &config.srcs,
            configuration.id,
            config.format,
            &config.feature_name,
            config.strand_specification,
            &config.data_type,
        )
        .await
    } else {
        import_from_paths(
            &mut tx,
            &config.srcs,
            configuration.id,
            &config.sample_name_delimiter,
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

#[allow(clippy::too_many_arguments)]
async fn import_from_paths<P>(
    tx: &mut Transaction<'_, Postgres>,
    srcs: &[P],
    configuration_id: i32,
    sample_name_delimiter: &str,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
    data_type: &str,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut chunk = Vec::with_capacity(srcs.len());

    for src in srcs {
        let path = src.as_ref();

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid filename"))?;
        // SAFETY: `str::Split` always has at least one item.
        let sample_name = filename.split(sample_name_delimiter).next().unwrap();

        let mut reader = File::open(path).await.map(BufReader::new)?;
        let counts = read_counts(&mut reader, format, feature_name, strand_specification).await?;

        chunk.push((sample_name.into(), counts));
    }

    import_batch(
        tx,
        configuration_id,
        strand_specification,
        data_type,
        &chunk,
    )
    .await?;

    Ok(())
}

async fn import_from_sample_sheets<P>(
    tx: &mut Transaction<'_, Postgres>,
    srcs: &[P],
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

    let mut chunk = Vec::new();

    for src in srcs {
        let f = File::open(src).await.map(BufReader::new)?;

        let mut lines = f.lines();

        while let Some(line) = lines.next_line().await? {
            let (sample_name, src) = line
                .split_once(DELIMITER)
                .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;

            let mut reader = File::open(src).await.map(BufReader::new)?;
            let counts =
                read_counts(&mut reader, format, feature_name, strand_specification).await?;

            chunk.push((sample_name.into(), counts));
        }
    }

    import_batch(
        tx,
        configuration_id,
        strand_specification,
        data_type,
        &chunk,
    )
    .await?;

    Ok(())
}

async fn import_batch(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    strand_specification: StrandSpecification,
    data_type: &str,
    chunk: &[(String, HashMap<String, u64>)],
) -> anyhow::Result<()> {
    use crate::store::{
        count::create_counts,
        feature::{create_features, find_features},
        run::{create_runs, runs_exists},
        sample::find_or_create_samples,
    };

    assert!(!chunk.is_empty());

    let mut features = find_features(&mut **tx, configuration_id).await?;

    info!("loaded {} features", features.len());

    if features.is_empty() {
        let mut names = HashSet::new();
        // SAFETY: `chunk` is non-empty.
        let (_, counts) = &chunk[0];
        names.extend(counts.keys().cloned());
        features = create_features(tx, configuration_id, &names).await?;
        info!("created {} features", features.len());
    }

    let sample_names: Vec<_> = chunk
        .iter()
        .map(|(sample_name, _)| sample_name.into())
        .collect();
    let sample_ids = find_or_create_samples(&mut **tx, &sample_names).await?;

    if runs_exists(&mut **tx, configuration_id, &sample_ids).await? {
        anyhow::bail!("run already exists for the sample and configuration");
    }

    let run_ids = create_runs(
        tx,
        configuration_id,
        &sample_ids,
        strand_specification,
        data_type,
    )
    .await?;

    for ((sample_name, counts), &run_id) in chunk.iter().zip(run_ids.iter()) {
        info!(name = sample_name, "loaded sample");

        if !feature_names_eq(&features, counts) {
            anyhow::bail!("feature name set mismatch");
        }

        create_counts(tx, run_id, &features, counts).await?;
    }

    info!(sample_count = sample_ids.len(), "imported samples");

    Ok(())
}
