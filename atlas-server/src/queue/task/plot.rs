mod error;
mod options;

use std::collections::HashMap;

use atlas_core::counts::dimension_reduction::tsne;
use sqlx::PgPool;

pub use self::{error::Error, options::Options};

struct Count {
    sample_name: String,
    feature_name: String,
    count: i32,
}

pub async fn plot(
    pool: &PgPool,
    dataset_id: i32,
    additional_runs: &[(String, HashMap<String, i32>)],
    options: Options,
) -> Result<(Vec<String>, Vec<f64>, Vec<f64>), Error> {
    use crate::store::{dataset, feature};

    let configuration_ids = dataset::configuration_ids(pool, dataset_id).await?;

    if configuration_ids.len() != 1 {
        return Err(Error::NonhomogeoneousDataset);
    }

    // SAFETY: `configuration_ids` is non-empty;
    let configuration_id = configuration_ids[0];

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

    if is_perplexity_too_large(options.perplexity, sample_count) {
        return Err(Error::PerplexityTooLarge {
            sample_count,
            perplexity: options.perplexity,
        });
    }

    let embedding = tsne::transform(options.perplexity, options.theta, raw_counts, feature_count);

    let mut xs = Vec::with_capacity(sample_count);
    let mut ys = Vec::with_capacity(sample_count);

    for chunk in embedding.chunks_exact(2) {
        xs.push(chunk[0]);
        ys.push(chunk[1]);
    }

    Ok((sample_names, xs, ys))
}

// See <https://github.com/frjnn/bhtsne/blob/a0dc63f7d967a748b9297a4108b1530e68eebf87/src/tsne/mod.rs#L46>.
fn is_perplexity_too_large(perplexity: f64, sample_count: usize) -> bool {
    let n = sample_count as f64;
    sample_count > 0 && (n - 1.0 < 3.0 * perplexity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_perplexity_too_large() {
        assert!(is_perplexity_too_large(30.0, 3));
        assert!(!is_perplexity_too_large(30.0, 100));
    }
}
