use std::num::ParseIntError;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use ndarray::{Array2, Axis};
use serde::{Deserialize, Serialize};

use crate::server::Error;

use super::Context;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Normalize {
    Fpkm,
    MedianOfRatios,
    Tpm,
}

#[derive(Debug, Deserialize)]
struct IndexQuery {
    run_ids: String,
    normalize: Option<Normalize>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Values {
    Normalized(Vec<f64>),
    Raw(Vec<i32>),
}

#[derive(Serialize)]
struct Run {
    id: i32,
    values: Values,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Counts {
    feature_names: Vec<String>,
    runs: Vec<Run>,
}

#[derive(Serialize)]
struct IndexBody {
    counts: Counts,
}

pub fn router() -> Router<Context> {
    Router::new().route("/counts", get(index))
}

/// Shows counts for runs.
#[utoipa::path(
    get,
    path = "/counts",
    operation_id = "counts-index",
    params(
        ("run_ids" = String, Query, description = "A comma-separated list of run IDs"),
    ),
    responses(
        (status = OK, description = "Counts associated with the given run IDs"),
    ),
)]
async fn index(
    State(ctx): State<Context>,
    Query(params): Query<IndexQuery>,
) -> super::Result<Json<IndexBody>> {
    use crate::{counts::normalization::median_of_ratios, store::feature};

    const DELIMITER: char = ',';

    let run_ids: Vec<i32> = params
        .run_ids
        .split(DELIMITER)
        .map(|s| {
            s.parse()
                .map_err(|e: ParseIntError| super::Error::Anyhow(e.into()))
        })
        .collect::<Result<_, _>>()?;

    if run_ids.is_empty() {
        return Err(Error::NotFound);
    }

    // SAFETY: `run_ids` is non-empty.
    let feature_names = feature::find_names_by_run_id(&ctx.pool, run_ids[0]).await?;

    let counts: Vec<_> = sqlx::query!(
        r#"
        select
            coalesce(counts.value, 0) as "value!"
        from runs
        inner join configurations
            on runs.configuration_id = configurations.id
        inner join features
            on runs.configuration_id = features.configuration_id
        left join counts
            on runs.id = counts.run_id and counts.feature_id = features.id
        where runs.id in (select unnest($1::integer[]))
        "#,
        &run_ids[..]
    )
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .map(|record| record.value)
    .collect();

    if counts.is_empty() {
        return Err(Error::NotFound);
    }

    let mut runs = Vec::with_capacity(run_ids.len());

    if let Some(normalization_method) = params.normalize {
        if matches!(normalization_method, Normalize::MedianOfRatios) {
            let values: Vec<_> = counts.into_iter().map(|n| n as u32).collect();
            let counts = Array2::from_shape_vec((run_ids.len(), feature_names.len()), values)
                .map_err(|e| super::Error::Anyhow(e.into()))?;

            let normalized_counts = median_of_ratios::normalize(counts);

            for (id, row) in run_ids
                .into_iter()
                .zip(normalized_counts.axis_iter(Axis(0)))
            {
                runs.push(Run {
                    id,
                    values: Values::Normalized(row.to_vec()),
                });
            }
        } else {
            let chunks = counts.chunks_exact(feature_names.len());

            // SAFETY: `run_ids` is non-empty.
            let features = feature::find_lengths_by_run_id(&ctx.pool, run_ids[0]).await?;

            for (id, chunk) in run_ids.into_iter().zip(chunks) {
                let counts = feature_names
                    .iter()
                    .zip(chunk)
                    .map(|(name, count)| (name.clone(), *count))
                    .collect();

                let normalized_counts_map = match normalization_method {
                    Normalize::Fpkm => {
                        crate::counts::normalization::fpkm::calculate_fpkms(&features, &counts)
                            .unwrap()
                    }
                    Normalize::MedianOfRatios => unreachable!(),
                    Normalize::Tpm => {
                        crate::counts::normalization::tpm::calculate_tpms(&features, &counts)
                            .unwrap()
                    }
                };

                let normalized_counts = feature_names
                    .iter()
                    .map(|name| normalized_counts_map[name])
                    .collect();

                runs.push(Run {
                    id,
                    values: Values::Normalized(normalized_counts),
                })
            }
        }
    } else {
        let chunks = counts.chunks_exact(feature_names.len());

        for (id, chunk) in run_ids.into_iter().zip(chunks) {
            runs.push(Run {
                id,
                values: Values::Raw(chunk.to_vec()),
            });
        }
    }

    Ok(Json(IndexBody {
        counts: Counts {
            feature_names,
            runs,
        },
    }))
}
