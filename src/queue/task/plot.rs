use sqlx::PgPool;
use thiserror::Error;

struct Count {
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

pub async fn plot(pool: &PgPool, configuration_id: i32) -> Result<(Vec<f32>, Vec<f32>), PlotError> {
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
            coalesce(counts.value, 0) as "count!"
        from runs
        inner join configurations
            on runs.configuration_id = configurations.id
        inner join features
            on runs.configuration_id = features.configuration_id
        left join counts
            on runs.id = counts.run_id and counts.feature_id = features.id
        where configurations.id = $1
        order by runs.id, features.name
        "#,
        configuration_id,
    )
    .fetch_all(pool)
    .await?;

    let raw_counts: Vec<_> = rows.into_iter().map(|count| count.count).collect();
    let sample_count = raw_counts.len();

    if sample_count - 1 < 3 * PERPLEXITY {
        return Err(PlotError::InsufficientSampleCount(sample_count));
    }

    let embedding = transform(raw_counts, feature_count);

    let mut xs = Vec::with_capacity(sample_count);
    let mut ys = Vec::with_capacity(sample_count);

    for chunk in embedding.chunks_exact(2) {
        xs.push(chunk[0]);
        ys.push(chunk[1]);
    }

    Ok((xs, ys))
}

fn transform(counts: Vec<i32>, feature_count: usize) -> Vec<f32> {
    #[cfg(not(test))]
    const PERPLEXITY: f32 = 30.0;

    #[cfg(test)]
    const PERPLEXITY: f32 = 3.0;

    const THETA: f32 = 0.5;

    fn euclidean_distance(a: &&[f32], b: &&[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(p, q)| (p - q).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    let sum: i32 = counts.iter().sum();

    let normalized_counts: Vec<_> = counts
        .into_iter()
        .map(|count| (count as f32) / (sum as f32))
        .collect();

    let data: Vec<_> = normalized_counts.chunks(feature_count).collect();

    bhtsne::tSNE::new(&data)
        .perplexity(PERPLEXITY)
        .barnes_hut(THETA, euclidean_distance)
        .embedding()
}
