use sqlx::PgPool;

struct Count {
    count: i32,
}

pub async fn plot(pool: &PgPool, configuration_id: i32) -> sqlx::Result<Vec<f32>> {
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
    .fetch_one(pool)
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
    .fetch_all(pool)
    .await?;

    let raw_counts: Vec<_> = rows.into_iter().map(|count| count.count).collect();
    let embedding = transform(raw_counts, feature_count);

    Ok(embedding)
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
