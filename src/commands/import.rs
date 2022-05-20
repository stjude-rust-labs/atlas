use std::collections::{HashMap, HashSet};

use crate::cli::ImportConfig;
use sqlx::postgres::PgPoolOptions;
use tokio::{
    fs::File,
    io::{AsyncBufRead, AsyncBufReadExt, BufReader},
};
use tracing::info;

pub async fn import(config: ImportConfig) -> anyhow::Result<()> {
    use crate::store::{
        annotations::find_or_create_annotations,
        configuration::find_or_create_configuration,
        count::create_counts,
        feature_name::{create_feature_names, find_feature_names},
        run::{create_run, run_exists},
        sample::find_or_create_sample,
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

    let sample = find_or_create_sample(&mut tx, &config.sample_name).await?;

    info!(id = sample.id, "loaded sample");

    if run_exists(&mut tx, configuration.id, sample.id).await? {
        tx.rollback().await?;
        anyhow::bail!("run already exists for the sample and configuration");
    }

    let mut feature_names = find_feature_names(&mut tx, configuration.id).await?;

    info!("loaded {} feature names", feature_names.len());

    let mut reader = File::open(&config.src).await.map(BufReader::new)?;
    let counts = read_feature_counts(&mut reader).await?;

    if feature_names.is_empty() {
        let mut names = HashSet::new();
        names.extend(counts.keys().cloned());
        feature_names = create_feature_names(&mut tx, configuration.id, &names).await?;
        info!("created {} feature names", feature_names.len());
    }

    let run = create_run(&mut tx, configuration.id, sample.id, &config.data_type).await?;
    create_counts(&mut tx, run.id, &feature_names, &counts).await?;

    tx.commit().await?;

    Ok(())
}

async fn read_feature_counts<R>(reader: &mut R) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    const DELIMITER: char = '\t';
    const HTSEQ_COUNT_META_PREFIX: &str = "__";

    let mut lines = reader.lines();
    let mut counts = HashMap::new();

    while let Some(line) = lines.next_line().await? {
        if let Some((raw_name, raw_count)) = line.split_once(DELIMITER) {
            if raw_name.starts_with(HTSEQ_COUNT_META_PREFIX) {
                break;
            }

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
        let data = b"feature_1\t8\nfeature_2\t13\n__no_feature\t0";

        let mut reader = &data[..];
        let counts = read_feature_counts(&mut reader).await?;

        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 8);
        assert_eq!(counts["feature_2"], 13);

        Ok(())
    }
}
