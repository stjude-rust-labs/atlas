use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::server::{self, Context, Error};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/configurations/{configuration_id}/features", get(index))
        .route(
            "/configurations/{configuration_id}/features/{id}",
            get(show),
        )
}

#[derive(Deserialize)]
struct IndexQuery {
    q: Option<String>,
}

#[derive(Serialize)]
struct IndexBody<T> {
    features: T,
}

#[derive(Serialize, sqlx::FromRow)]
struct Feature {
    id: i32,
    name: String,
    length: i32,
}

/// Lists features in a configuration.
#[utoipa::path(
    get,
    path = "/configurations/{configuration_id}/features",
    operation_id = "configurations-features-index",
    params(
        ("configuration_id" = i32, Path, description = "Configuration ID"),
        ("q" = String, Query, description = "A search pattern of the feature name"),
    ),
    responses(
        (status = OK, description = "Features associated with the given configuration"),
        (status = NOT_FOUND, description = "The configuration ID does not exist"),
    ),
)]
async fn index(
    Path(configuration_id): Path<i32>,
    Query(params): Query<IndexQuery>,
    State(ctx): State<Context>,
) -> server::Result<Json<IndexBody<Vec<Feature>>>> {
    use crate::store::configuration;

    if !configuration::exists(&ctx.pool, configuration_id).await? {
        return Err(Error::NotFound);
    }

    let features = if let Some(q) = params.q {
        sqlx::query_as(
            r#"
            select
                id,
                name,
                length
            from features
            where configuration_id = $1
                and name ilike concat('%', $2, '%')
            "#,
        )
        .bind(configuration_id)
        .bind(q)
        .fetch_all(&ctx.pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            select
                id,
                name,
                length
            from features
            where configuration_id = $1
            "#,
        )
        .bind(configuration_id)
        .fetch_all(&ctx.pool)
        .await?
    };

    Ok(Json(IndexBody { features }))
}

#[derive(Serialize)]
struct Run {
    id: i32,
    count: i32,
}

#[derive(Serialize)]
struct ShowFeature {
    id: i32,
    name: String,
    runs: Vec<Run>,
}

#[derive(Serialize)]
struct ShowResponse {
    feature: ShowFeature,
}

/// Shows counts for samples with the given configuration ID and feature name.
#[utoipa::path(
    get,
    path = "/configurations/{configuration_id}/features/{id}",
    operation_id = "configurations-features-show",
    params(
        ("configuration_id" = i32, Path, description = "Configuration ID"),
        ("id" = i32, Path, description = "Feature ID"),
    ),
    responses(
        (status = OK, description = "Counts associated with the given configuration ID and feature ID"),
    ),
)]
async fn show(
    Path((configuration_id, id)): Path<(i32, i32)>,
    State(ctx): State<Context>,
) -> server::Result<Json<ShowResponse>> {
    let rows = sqlx::query!(
        "
        select
            features.name,
            runs.id,
            counts.value
        from counts
        inner join features
            on features.id = counts.feature_id
        inner join runs
            on runs.id = counts.run_id
        inner join samples
            on samples.id = runs.sample_id
        where counts.feature_id = $1
            and runs.configuration_id = $2
        ",
        id,
        configuration_id,
    )
    .fetch_all(&ctx.pool)
    .await?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

    // SAFETY: `rows` is non-empty.
    let name = rows[0].name.clone();

    let runs = rows
        .into_iter()
        .map(|row| Run {
            id: row.id,
            count: row.value,
        })
        .collect();

    Ok(Json(ShowResponse {
        feature: ShowFeature { id, name, runs },
    }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
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
                    "length": 8,
                }, {
                    "id": 2,
                    "name": "39_feature_2",
                    "length": 13,
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

    #[sqlx::test(fixtures("features"))]
    async fn test_show(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/configurations/1/features/1")
            .body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "feature": {
                    "id": 1,
                    "name": "39_feature_1",
                    "runs": [{
                        "id": 1,
                        "count": 5,
                    }, {
                        "id": 2,
                        "count": 13,
                    }],
                }
            })
        );

        Ok(())
    }
}
