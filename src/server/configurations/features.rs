use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::server::{self, Context, Error};

pub fn router() -> Router<Context> {
    Router::new().route("/configurations/:configuration_id/features", get(index))
}

#[derive(Serialize)]
struct IndexBody<T> {
    features: T,
}

#[derive(Serialize, sqlx::FromRow)]
struct Feature {
    id: i32,
    name: String,
}

/// Lists features in a configuration.
#[utoipa::path(
    get,
    path = "/configurations/{configuration_id}/features",
    params(
        ("configuration_id" = i32, Path, description = "Configuration ID"),
    ),
    responses(
        (status = OK, description = "Features associated with the given configuration"),
        (status = NOT_FOUND, description = "The configuration ID does not exist"),
    ),
)]
async fn index(
    Path(configuration_id): Path<i32>,
    State(ctx): State<Context>,
) -> server::Result<Json<IndexBody<Vec<Feature>>>> {
    use crate::store::configuration;

    if !configuration::exists(&ctx.pool, configuration_id).await? {
        return Err(Error::NotFound);
    }

    let features = sqlx::query_as(
        r#"
        select
            id,
            name
        from feature_names
        where configuration_id = $1
        "#,
    )
    .bind(configuration_id)
    .fetch_all(&ctx.pool)
    .await?;

    Ok(Json(IndexBody { features }))
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

    #[sqlx::test(fixtures("features"))]
    async fn test_index(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/configurations/1/features")
            .body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "features": [{
                    "id": 1,
                    "name": "39_feature_1",
                }, {
                    "id": 2,
                    "name": "39_feature_2",
                }]
            })
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_index_with_invalid_configuration_id(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/configurations/1/features")
            .body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }
}
