use std::{collections::HashMap, path::Path};

use atlas_core::features::{Feature, calculate_feature_lengths};
use sqlx::postgres::PgPoolOptions;
use tokio::io;
use tracing::info;

use crate::{cli::configuration::ImportConfig, store::feature::create_features};

pub(super) async fn import(config: ImportConfig) -> anyhow::Result<()> {
    use crate::store::{annotations::find_or_create_annotations, configuration};

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

    let configuration_id = configuration::create(
        &mut tx,
        annotations.id,
        &config.feature_type,
        &config.feature_name,
    )
    .await?;

    info!(id = configuration_id, "imported configuration");

    let features = read_features(&config.src, &config.feature_type, &config.feature_name).await?;

    let mut names: Vec<_> = features.keys().cloned().collect();
    names.sort();

    let lengths: Vec<_> = calculate_feature_lengths(&features, &names)?
        .into_iter()
        .map(i32::try_from)
        .collect::<Result<_, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    create_features(&mut tx, configuration_id, &names, &lengths).await?;

    info!("imported {} features", names.len());

    tx.commit().await?;

    println!("{}", configuration_id);

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

        use atlas_core::features::read_features;

        let mut reader = File::open(src).map(BufReader::new)?;

        let (_, features) = read_features(&mut reader, &feature_type, &feature_name)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(features)
    })
    .await?
}
