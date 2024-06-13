use std::{collections::HashMap, path::Path};

use sqlx::postgres::PgPoolOptions;
use tokio::io;
use tracing::info;

use crate::{
    cli::configuration::ImportConfig,
    features::{merge_features, Feature},
    store::feature::create_features,
};

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

    let features = read_features(&config.src, &config.feature_type, &config.feature_name).await?;

    let mut names: Vec<_> = features.keys().cloned().collect();
    names.sort();

    let lengths = calculate_feature_lengths(&features, &names)?;

    create_features(&mut tx, configuration.id, &names, &lengths).await?;

    info!("imported {} features", names.len());

    tx.commit().await?;

    Ok(())
}

async fn read_features<P>(
    src: P,
    feature_type: &str,
    feature_name: &str,
) -> io::Result<HashMap<String, Vec<Feature>>>
where
    P: AsRef<Path>,
{
    let src = src.as_ref().to_path_buf();
    let feature_type = feature_type.to_owned();
    let feature_name = feature_name.to_owned();

    tokio::task::spawn_blocking(move || {
        use std::{
            fs::File,
            io::{self, BufReader},
        };

        use crate::features::read_features;

        let mut reader = File::open(src).map(BufReader::new)?;

        read_features(&mut reader, &feature_type, &feature_name)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    })
    .await?
}

fn calculate_feature_lengths(
    features: &HashMap<String, Vec<Feature>>,
    names: &[String],
) -> io::Result<Vec<i32>> {
    let mut lengths = Vec::with_capacity(names.len());

    for name in names {
        let segments = features.get(name).unwrap();
        let merged_segments = merge_features(segments);

        let length: usize = merged_segments
            .into_iter()
            .map(|(start, end)| usize::from(end) - usize::from(start) + 1)
            .sum();

        let length =
            i32::try_from(length).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        lengths.push(length);
    }

    Ok(lengths)
}
