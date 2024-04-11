mod options;

use std::collections::HashMap;

use sqlx::PgPool;
use thiserror::Error;

pub use self::options::Options;

struct Count {
    sample_name: String,
    feature_name: String,
    count: i32,
}

#[derive(Debug, Error)]
pub enum PlotError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("insufficient number of samples: got {0}, expected > 3 * {PERPLEXITY}")]
    InsufficientSampleCount(usize),
}

#[cfg(not(test))]
const PERPLEXITY: usize = 30;

#[cfg(test)]
const PERPLEXITY: usize = 3;

pub async fn plot(
    pool: &PgPool,
    configuration_id: i32,
    additional_runs: &[(String, HashMap<String, i32>)],
    options: Options,
) -> Result<(Vec<String>, Vec<f64>, Vec<f64>), PlotError> {
    use crate::store::feature;

    let feature_count = feature::count(pool, configuration_id).await? as usize;

    let rows = sqlx::query_as!(
        Count,
        r#"
        select
            samples.name as sample_name,
            features.name as feature_name,
            coalesce(counts.value, 0) as "count!"
        from runs
        inner join configurations
            on runs.configuration_id = configurations.id
        inner join features
            on runs.configuration_id = features.configuration_id
        inner join samples
            on runs.sample_id = samples.id
        left join counts
            on runs.id = counts.run_id and counts.feature_id = features.id
        where configurations.id = $1
        order by runs.id, features.id
        "#,
        configuration_id,
    )
    .fetch_all(pool)
    .await?;

    let mut sample_names = Vec::new();

    for chunk in rows.chunks_exact(feature_count) {
        let Count { sample_name, .. } = &chunk[0];
        sample_names.push(sample_name.into());
    }

    let feature_names: Vec<_> = rows[..feature_count]
        .iter()
        .map(|count| count.feature_name.clone())
        .collect();

    let mut raw_counts: Vec<_> = rows.into_iter().map(|count| count.count).collect();

    for (sample_name, counts) in additional_runs {
        raw_counts.extend(feature_names.iter().map(|name| counts[name]));
        sample_names.push(sample_name.into());
    }

    let sample_count = sample_names.len();

    if sample_count - 1 < 3 * PERPLEXITY {
        return Err(PlotError::InsufficientSampleCount(sample_count));
    }

    let embedding = transform(options.perplexity, options.theta, raw_counts, feature_count);

    let mut xs = Vec::with_capacity(sample_count);
    let mut ys = Vec::with_capacity(sample_count);

    for chunk in embedding.chunks_exact(2) {
        xs.push(chunk[0]);
        ys.push(chunk[1]);
    }

    Ok((sample_names, xs, ys))
}

fn transform(perplexity: f64, theta: f64, counts: Vec<i32>, feature_count: usize) -> Vec<f64> {
    fn euclidean_distance(a: &&[f64], b: &&[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(p, q)| (p - q).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    let sum: u64 = counts.iter().map(|n| *n as u64).sum();

    let normalized_counts: Vec<_> = counts
        .into_iter()
        .map(|count| (count as f64) / (sum as f64))
        .collect();

    let data: Vec<_> = normalized_counts.chunks(feature_count).collect();

    bhtsne::tSNE::new(&data)
        .perplexity(perplexity)
        .barnes_hut(theta, euclidean_distance)
        .embedding()
}
