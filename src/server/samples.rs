pub mod runs;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use time::OffsetDateTime;

use crate::store::sample;

use super::{Context, Error};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/samples", get(index))
        .route("/samples/:id", get(show))
}

#[derive(Serialize, utoipa::ToSchema)]
struct IndexResponse {
    #[schema(inline)]
    samples: Vec<Sample>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Sample {
    id: i32,
    name: String,
    #[schema(inline)]
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

/// Lists all samples with runs.
#[utoipa::path(
    get,
    path = "/samples",
    operation_id = "samples-index",
    responses(
        (status = OK, description = "Samples with runs", body = inline(IndexResponse)),
    )
)]
async fn index(State(ctx): State<Context>) -> super::Result<Json<IndexResponse>> {
    let samples = sqlx::query_as!(Sample, r#"select id, name, created_at from samples"#)
        .fetch_all(&ctx.pool)
        .await?;

    Ok(Json(IndexResponse { samples }))
}

#[derive(Serialize, utoipa::ToSchema)]
struct ShowResponse {
    #[schema(inline)]
    sample: sample::Sample,
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
        (status = OK, description = "The sample has runs", body = inline(ShowResponse)),
        (status = NOT_FOUND, description = "The sample does not exist")
    ),
)]
async fn show(
    State(ctx): State<Context>,
    Path(id): Path<i32>,
) -> super::Result<Json<ShowResponse>> {
    let sample = sample::find(&ctx.pool, id).await?.ok_or(Error::NotFound)?;
    Ok(Json(ShowResponse { sample }))
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
