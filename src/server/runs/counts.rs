use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::server::{self, Context, Error};

pub fn router() -> Router<Context> {
    Router::new().route("/runs/:run_id/counts", get(index))
}

#[derive(Serialize)]
struct IndexBody {
    counts: HashMap<String, i32>,
}

struct Count {
    name: String,
    value: i32,
}

/// Shows counts for a given run.
#[utoipa::path(
    get,
    path = "/runs/{run_id}",
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
) -> server::Result<Json<IndexBody>> {
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

    let counts = rows.into_iter().map(|c| (c.name, c.value)).collect();

    Ok(Json(IndexBody { counts }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde::Deserialize;
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
        #[derive(Deserialize)]
        struct CountsBody {
            counts: HashMap<String, i32>,
        }

        let request = Request::builder()
            .uri("/runs/1/counts")
            .body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await?.to_bytes();
        let actual: CountsBody = serde_json::from_slice(&body)?;

        let expected = [("feature_1".into(), 8), ("feature_2".into(), 0)]
            .into_iter()
            .collect();

        assert_eq!(actual.counts, expected);

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
