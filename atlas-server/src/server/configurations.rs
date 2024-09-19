pub mod features;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use super::{Context, Error};
use crate::store::configuration::{self, Configuration};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/configurations", get(index))
        .route("/configurations/:id", get(show))
}

#[derive(Serialize, utoipa::ToSchema)]
struct IndexResponse {
    #[schema(inline)]
    configurations: Vec<Configuration>,
}

/// Lists all configurations.
#[utoipa::path(
    get,
    path = "/configurations",
    operation_id = "configurations-index",
    responses(
        (status = OK, description = "A list of configurations", body = inline(IndexResponse)),
    ),
)]
async fn index(State(ctx): State<Context>) -> super::Result<Json<IndexResponse>> {
    let configurations = configuration::all(&ctx.pool).await?;
    Ok(Json(IndexResponse { configurations }))
}

#[derive(Serialize)]
struct ShowResponse {
    configuration: Configuration,
}

/// Find a configuration by ID.
#[utoipa::path(
    get,
    path = "/configurations/{id}",
    operation_id = "configurations-show",
    params(
        ("id" = i32, Path, description = "Configuration ID"),
    ),
    responses(
        (status = OK, description = "The configurations of the given ID"),
        (status = NOT_FOUND, description = "The configuration does not exist"),
    ),
)]
async fn show(
    State(ctx): State<Context>,
    Path(id): Path<i32>,
) -> super::Result<Json<ShowResponse>> {
    let configuration = configuration::find(&ctx.pool, id)
        .await?
        .ok_or(Error::NotFound)?;

    Ok(Json(ShowResponse { configuration }))
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

    #[sqlx::test(fixtures("configurations"))]
    async fn test_index(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/configurations")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "configurations": [{
                    "id": 1,
                    "annotations": {
                        "name": "GENCODE 39",
                        "genomeBuild": "GRCh38.p13",
                    },
                    "featureType": "exon",
                    "featureName": "gene_name",
                }, {
                    "id": 2,
                    "annotations": {
                        "name": "GENCODE 19",
                        "genomeBuild": "GRCh37.p13",
                    },
                    "featureType": "exon",
                    "featureName": "gene_name",
                }]
            })
        );

        Ok(())
    }

    #[sqlx::test(fixtures("configurations"))]
    async fn test_show(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/configurations/3")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
