use std::collections::HashMap;

use sqlx::{postgres::PgPoolOptions, Postgres, Transaction};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tracing::info;

use crate::cli::ImportConfig;

pub async fn import(config: ImportConfig) -> anyhow::Result<()> {
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

    let configuration = create_configuration(
        &mut tx,
        annotations.id,
        &config.feature_type,
        &config.feature_name,
    )
    .await?;

    info!(id = configuration.id, "loaded configuration");

    let sample = find_or_create_sample(&mut tx, &config.sample_name).await?;

    info!(id = sample.id, "loaded sample");

    if run_exists(&mut tx, configuration.id, sample.id).await? {
        tx.rollback().await?;
        anyhow::bail!("run already exists for the sample and configuration");
    }

    tx.commit().await?;

    Ok(())
}

#[derive(Debug)]
struct Annotations {
    id: i32,
}

async fn find_or_create_annotations(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    genome_build: &str,
) -> anyhow::Result<Annotations> {
    let annotations_id = sqlx::query_scalar!(
        "
        insert into annotations
            (name, genome_build)
        values
            ($1, $2)
        on conflict (name) do update
            set id = annotations.id
        returning id
        ",
        name,
        genome_build,
    )
    .fetch_one(tx)
    .await?;

    Ok(Annotations { id: annotations_id })
}

#[derive(Debug)]
struct Configuration {
    id: i32,
}

async fn create_configuration(
    tx: &mut Transaction<'_, Postgres>,
    annotations_id: i32,
    feature_type: &str,
    feature_name: &str,
) -> anyhow::Result<Configuration> {
    let configuration_id = sqlx::query_scalar!(
        "
        insert into configurations
            (annotation_id, feature_type, feature_name)
        values
            ($1, $2, $3)
        returning id
        ",
        annotations_id,
        feature_type,
        feature_name,
    )
    .fetch_one(tx)
    .await?;

    Ok(Configuration {
        id: configuration_id,
    })
}

#[derive(Debug)]
struct Sample {
    id: i32,
}

async fn find_or_create_sample(
    tx: &mut Transaction<'_, Postgres>,
    sample_name: &str,
) -> anyhow::Result<Sample> {
    let sample_id = sqlx::query_scalar!(
        "
        insert into samples (name) values ($1)
        on conflict (name) do update
            set id = samples.id
        returning id
        ",
        sample_name
    )
    .fetch_one(tx)
    .await?;

    Ok(Sample { id: sample_id })
}

async fn run_exists(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    sample_id: i32,
) -> anyhow::Result<bool> {
    sqlx::query_scalar!(
        "
        select 1
        from runs
        where configuration_id = $1 and sample_id = $2
        limit 1
        ",
        configuration_id,
        sample_id,
    )
    .fetch_optional(tx)
    .await
    .map(|result| result.is_some())
    .map_err(|e| e.into())
}

#[allow(dead_code)]
async fn read_feature_counts<R>(reader: &mut R) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    const TAB: char = '\t';

    let mut lines = reader.lines();
    let mut counts = HashMap::new();

    while let Some(line) = lines.next_line().await? {
        if let Some((raw_name, raw_count)) = line.split_once(TAB) {
            let count = raw_count.parse()?;
            counts.insert(raw_name.into(), count);
        }
    }

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_feature_counts() -> anyhow::Result<()> {
        let data = b"feature_1\t8\nfeature_2\t13\n";

        let mut reader = &data[..];
        let counts = read_feature_counts(&mut reader).await?;

        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 8);
        assert_eq!(counts["feature_2"], 13);

        Ok(())
    }
}
