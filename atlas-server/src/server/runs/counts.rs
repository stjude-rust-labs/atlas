use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::server::{self, Context, Error};

pub fn router() -> Router<Context> {
    Router::new().route("/runs/:run_id/counts", get(index))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Normalize {
    Fpkm,
    MedianOfRatios,
    Tpm,
}

#[derive(Debug, Deserialize)]
struct IndexQuery {
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
    run: Run,
}

#[derive(Serialize)]
struct IndexBody {
    counts: Counts,
}

struct Count {
    name: String,
    value: i32,
}

/// Shows counts for a given run.
#[utoipa::path(
    get,
    path = "/runs/{run_id}/counts",
    operation_id = "runs-counts-index",
    params(
        ("run_id" = i32, Path, description = "Run ID"),
    ),
    responses(
        (status = OK, description = "Counts associated with the given run"),
        (status = NOT_FOUND, description = "The run ID does not exist")
    )
)]
async fn index(
    State(ctx): State<Context>,
    Path(run_id): Path<i32>,
    Query(params): Query<IndexQuery>,
) -> server::Result<Json<IndexBody>> {
    use crate::store::feature;

    let rows = sqlx::query_as!(
        Count,
        r#"
        select
            features.name,
            coalesce(counts.value, 0) as "value!"
        from runs
        inner join configurations
            on runs.configuration_id = configurations.id
        inner join features
            on runs.configuration_id = features.configuration_id
        left join counts
            on runs.id = counts.run_id and counts.feature_id = features.id
        where runs.id = $1
        "#,
        run_id
    )
    .fetch_all(&ctx.pool)
    .await?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

    let feature_names: Vec<_> = rows.iter().map(|row| row.name.clone()).collect();
    let counts = rows.into_iter().map(|row| (row.name, row.value)).collect();

    let values = if let Some(normalization_method) = params.normalize {
        let features = feature::find_lengths_by_run_id(&ctx.pool, run_id).await?;

        let normalized_counts = match normalization_method {
            Normalize::Fpkm => {
                atlas_core::counts::normalization::fpkm::calculate_fpkms_map(&features, &counts)
                    .unwrap()
            }
            Normalize::MedianOfRatios => {
                // Applying median of ratios to a single sample is a no-op.
                counts
                    .into_iter()
                    .map(|(name, value)| (name, f64::from(value)))
                    .collect()
            }
            Normalize::Tpm => {
                atlas_core::counts::normalization::tpm::calculate_tpms(&features, &counts).unwrap()
            }
        };

        let values = feature_names
            .iter()
            .map(|name| normalized_counts[name])
            .collect();

        Values::Normalized(values)
    } else {
        let values = feature_names.iter().map(|name| counts[name]).collect();
        Values::Raw(values)
    };

    Ok(Json(IndexBody {
        counts: Counts {
            feature_names,
            run: Run { id: run_id, values },
        },
    }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use sqlx::PgPool;
    use tower::ServiceExt;

    use super::*;
    use crate::Queue;

    fn app(pool: PgPool) -> Router {
        let queue = Queue::new(pool.clone());
        router().with_state(Context { pool, queue })
    }

    #[sqlx::test(fixtures("counts"))]
    async fn test_show(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/runs/1/counts")
            .body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "counts": {
                    "featureNames": ["feature_1", "feature_2"],
                    "run": {
                        "id": 1,
                        "values": [8, 0],
                    },
                },
            })
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_show_with_an_invalid_id(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/runs/1597/counts")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
