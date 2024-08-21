use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use time::OffsetDateTime;

use crate::store::StrandSpecification;

use super::{Context, Error};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/samples", get(index))
        .route("/samples/:id", get(show))
}

#[derive(Serialize)]
struct SamplesBody<T> {
    samples: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Sample {
    id: i32,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

/// Lists all samples with runs.
#[utoipa::path(
    get,
    path = "/samples",
    operation_id = "samples-index",
    responses(
        (status = OK, description = "Samples with runs"),
    )
)]
async fn index(State(ctx): State<Context>) -> super::Result<Json<SamplesBody<Vec<Sample>>>> {
    let samples = sqlx::query_as!(Sample, r#"select id, name, created_at from samples"#)
        .fetch_all(&ctx.pool)
        .await?;

    Ok(Json(SamplesBody { samples }))
}

struct SampleFromQuery {
    sample_id: i32,
    sample_name: String,
    counts_id: i32,
    counts_data_type: String,
    counts_configuration_id: i32,
    counts_strand_specification: StrandSpecification,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    id: i32,
    data_type: String,
    configuration_id: i32,
    strand_specification: StrandSpecification,
}

#[derive(Serialize)]
struct SampleWithCounts {
    id: i32,
    name: String,
    runs: Vec<Run>,
}

#[derive(Serialize)]
struct ShowResponse {
    sample: SampleWithCounts,
}

/// Shows associated runs for a given sample.
#[utoipa::path(
    get,
    path = "/samples/{id}",
    operation_id = "samples-show",
    params(
        ("id" = i32, Path, description = "Sample ID"),
    ),
    responses(
        (status = OK, description = "The sample has runs"),
        (status = NOT_FOUND, description = "The sample does not exist")
    ),
)]
async fn show(
    State(ctx): State<Context>,
    Path(id): Path<i32>,
) -> super::Result<Json<ShowResponse>> {
    let rows = sqlx::query_as!(
        SampleFromQuery,
        r#"
            select
                samples.id as sample_id,
                samples.name as sample_name,
                runs.id as counts_id,
                runs.configuration_id as counts_configuration_id,
                runs.strand_specification as "counts_strand_specification: _",
                runs.data_type as counts_data_type
            from samples
            inner join runs
                on runs.sample_id = samples.id
            inner join configurations
                on runs.configuration_id = configurations.id
            where samples.id = $1
        "#,
        id
    )
    .fetch_all(&ctx.pool)
    .await?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

    let first_row = rows.first().expect("missing first row");
    let id = first_row.sample_id;
    let name = first_row.sample_name.clone();

    let runs = rows
        .into_iter()
        .map(|row| Run {
            id: row.counts_id,
            data_type: row.counts_data_type,
            configuration_id: row.counts_configuration_id,
            strand_specification: row.counts_strand_specification,
        })
        .collect();

    Ok(Json(ShowResponse {
        sample: SampleWithCounts { id, name, runs },
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

    #[sqlx::test(fixtures("samples"))]
    async fn test_index(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder().uri("/samples").body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "samples": [{
                    "id": 1,
                    "name": "sample_1",
                    "createdAt": "2022-02-18T21:05:05Z",
                }, {
                    "id": 2,
                    "name": "sample_2",
                    "createdAt": "2022-02-18T21:05:06Z",
                }]
            })
        );

        Ok(())
    }

    #[sqlx::test(fixtures("samples"))]
    async fn test_show(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder().uri("/samples/1").body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;
        let body = response.into_body().collect().await?.to_bytes();
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "sample": {
                    "id": 1,
                    "name": "sample_1",
                    "runs": [{
                        "id": 1,
                        "configurationId": 1,
                        "strandSpecification": "reverse",
                        "dataType": "RNA-Seq",
                    }, {
                        "id": 2,
                        "configurationId": 2,
                        "strandSpecification": "reverse",
                        "dataType": "RNA-Seq",
                    }],
                },
            })
        );

        Ok(())
    }

    #[sqlx::test(fixtures("samples"))]
    async fn test_show_with_an_invalid_name(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/samples/1597")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
