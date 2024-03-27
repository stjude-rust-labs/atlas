use sqlx::PgPool;
use thiserror::Error;

struct Count {
    sample_name: String,
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
) -> Result<(Vec<String>, Vec<f64>, Vec<f64>), PlotError> {
    let feature_count = sqlx::query!(
        r#"
        select
            count(*) as "count!"
        from
            features
        where
            configuration_id = $1
        "#,
        configuration_id
    )
    .fetch_one(pool)
    .await
    .map(|record| record.count as usize)?;

    let rows = sqlx::query_as!(
        Count,
        r#"
        select
            samples.name as sample_name,
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
        order by runs.id, features.name
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

    let sample_count = sample_names.len();

    if sample_count - 1 < 3 * PERPLEXITY {
        return Err(PlotError::InsufficientSampleCount(sample_count));
    }

    let raw_counts: Vec<_> = rows.into_iter().map(|count| count.count).collect();
    let embedding = transform(raw_counts, feature_count);

    let mut xs = Vec::with_capacity(sample_count);
    let mut ys = Vec::with_capacity(sample_count);

    for chunk in embedding.chunks_exact(2) {
        xs.push(chunk[0]);
        ys.push(chunk[1]);
    }

    Ok((sample_names, xs, ys))
}

fn transform(counts: Vec<i32>, feature_count: usize) -> Vec<f64> {
    #[cfg(not(test))]
    const PERPLEXITY: f64 = 30.0;

    #[cfg(test)]
    const PERPLEXITY: f64 = 3.0;

    const THETA: f64 = 0.5;

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
        .perplexity(PERPLEXITY)
        .barnes_hut(THETA, euclidean_distance)
        .embedding()
}
