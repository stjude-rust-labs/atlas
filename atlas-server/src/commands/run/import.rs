use std::{collections::HashMap, io, path::Path};

use sqlx::{postgres::PgPoolOptions, Postgres, Transaction};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tracing::info;

use crate::{
    cli::run::ImportConfig,
    counts::{feature_names_eq, Format},
    store::{dataset, StrandSpecification},
};

const BATCH_CHUNK_SIZE: usize = 128;

pub async fn import(config: ImportConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let mut tx = pool.begin().await?;

    let configuration_id = config.configuration_id;

    let feature_name = sqlx::query!(
        "select feature_name from configurations where id = $1",
        configuration_id
    )
    .fetch_one(&pool)
    .await
    .map(|record| record.feature_name)?;

    info!(src_count = config.srcs.len(), "reading srcs");

    let result = if config.sample_sheet {
        import_from_sample_sheets(
            &mut tx,
            &config.srcs,
            configuration_id,
            config.dataset_id,
            config.format,
            &feature_name,
            config.strand_specification,
            &config.data_type,
        )
        .await
    } else {
        import_from_paths(
            &mut tx,
            &config.srcs,
            configuration_id,
            config.dataset_id,
            &config.sample_name_delimiter,
            config.format,
            &feature_name,
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
    dataset_id: Option<i32>,
    sample_name_delimiter: &str,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
    data_type: &str,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    for src_chunk in srcs.chunks(BATCH_CHUNK_SIZE) {
        let mut chunk = Vec::with_capacity(src_chunk.len());

        for src in src_chunk {
            let path = src.as_ref();

            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("invalid filename"))?;
            // SAFETY: `str::Split` always has at least one item.
            let sample_name = filename.split(sample_name_delimiter).next().unwrap();

            let counts = read_counts(path, format, feature_name, strand_specification).await?;

            chunk.push((sample_name.into(), counts));
        }

        import_batch(
            tx,
            configuration_id,
            dataset_id,
            strand_specification,
            data_type,
            &chunk,
        )
        .await?;
    }

    info!(sample_count = srcs.len(), "imported samples");

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn import_from_sample_sheets<P>(
    tx: &mut Transaction<'_, Postgres>,
    srcs: &[P],
    configuration_id: i32,
    dataset_id: Option<i32>,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
    data_type: &str,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    const DELIMITER: char = '\t';

    let mut sample_count = 0;

    for src_chunk in srcs.chunks(BATCH_CHUNK_SIZE) {
        let mut chunk = Vec::new();

        for src in src_chunk {
            let f = File::open(src).await.map(BufReader::new)?;

            let mut lines = f.lines();

            while let Some(line) = lines.next_line().await? {
                let (sample_name, src) = line
                    .split_once(DELIMITER)
                    .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;

                let counts = read_counts(src, format, feature_name, strand_specification).await?;

                chunk.push((sample_name.into(), counts));

                sample_count += 1;
            }

            import_batch(
                tx,
                configuration_id,
                dataset_id,
                strand_specification,
                data_type,
                &chunk,
            )
            .await?;
        }
    }

    info!(sample_count, "imported samples");

    Ok(())
}

async fn import_batch(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    dataset_id: Option<i32>,
    strand_specification: StrandSpecification,
    data_type: &str,
    chunk: &[(String, HashMap<String, u64>)],
) -> anyhow::Result<()> {
    use crate::store::{
        count::create_counts,
        feature::find_features,
        run::{create_runs, runs_exists},
        sample::find_or_create_samples,
    };

    assert!(!chunk.is_empty());

    let features = find_features(&mut **tx, configuration_id).await?;

    if features.is_empty() {
        anyhow::bail!("configuration {configuration_id} is missing features");
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

    if let Some(dataset_id) = dataset_id {
        dataset::create_runs(&mut **tx, dataset_id, &run_ids).await?;
    }

    for ((sample_name, counts), &run_id) in chunk.iter().zip(run_ids.iter()) {
        info!(name = sample_name, "loaded sample");

        if !feature_names_eq(&features, counts) {
            anyhow::bail!("feature name set mismatch");
        }

        create_counts(tx, run_id, &features, counts).await?;
    }

    Ok(())
}

async fn read_counts<P>(
    src: P,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
) -> anyhow::Result<HashMap<String, u64>>
where
    P: AsRef<Path>,
{
    let src = src.as_ref().to_path_buf();
    let feature_name = feature_name.to_owned();

    let counts = tokio::task::spawn_blocking(move || {
        let mut reader = std::fs::File::open(src).map(std::io::BufReader::new)?;

        atlas_core::counts::reader::read(
            &mut reader,
            format.map(|f| f.into()),
            &feature_name,
            strand_specification.into(),
        )
    })
    .await??;

    Ok(counts.into_iter().collect())
}
