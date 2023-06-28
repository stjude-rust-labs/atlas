use std::fmt::Write;

use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};

use super::Context;

pub fn router() -> Router<Context> {
    Router::new().route("/analyses/plot/:configuration_id", get(plot))
}

struct Count {
    count: i32,
}

/// Performs dimension reduction on all samples in a configuation.
#[utoipa::path(
    get,
    path = "/analyses/plot/{configuration-id}",
    params(
        ("configuration-id" = i32, Path, description = "Configuration ID"),
    ),
    responses(
        (status = OK, description = "Coordinates in two dimensions"),
        (status = NOT_FOUND, description = "The configuration ID does not exist"),
    ),
)]
async fn plot(
    State(ctx): State<Context>,
    Path(configuration_id): Path<i32>,
) -> super::Result<String> {
    let feature_count = sqlx::query!(
        r#"
        select
            count(*) as "count!"
        from
            feature_names
        where
            configuration_id = $1
        "#,
        configuration_id
    )
    .fetch_one(&ctx.pool)
    .await
    .map(|record| record.count as usize)?;

    let rows = sqlx::query_as!(
        Count,
        r#"
        select
            counts.value as count
        from
            counts
        inner join runs
            on counts.run_id = runs.id
        where
            runs.configuration_id = $1
        "#,
        configuration_id,
    )
    .fetch_all(&ctx.pool)
    .await?;

    let raw_counts: Vec<_> = rows.into_iter().map(|count| count.count).collect();
    let embedding = transform(raw_counts, feature_count);

    let mut body = String::new();

    for p in embedding.chunks_exact(2) {
        let (x, y) = (p[0], p[1]);
        writeln!(body, "{x},{y}").map_err(anyhow::Error::new)?;
    }

    Ok(body)
}

fn transform(counts: Vec<i32>, feature_count: usize) -> Vec<f32> {
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
