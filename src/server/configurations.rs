pub mod features;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use super::Context;
use crate::store::configuration;

pub fn router() -> Router<Context> {
    Router::new().route("/configurations", get(index))
}

#[derive(Serialize, utoipa::ToSchema)]
struct IndexResponse {
    #[schema(inline)]
    configurations: Vec<configuration::AllResult>,
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
                    "annotation": {
                        "name": "GENCODE 39",
                        "genomeBuild": "GRCh38.p13",
                    },
                    "featureType": "exon",
                    "featureName": "gene_name",
                }, {
                    "id": 2,
                    "annotation": {
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
}
