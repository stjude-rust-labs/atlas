use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::server::{self, Context};

pub fn router() -> Router<Context> {
    Router::new().route("/features/:feature_id/runs", get(index))
}

#[derive(Serialize)]
struct IndexBody<T> {
    runs: T,
}

#[derive(Serialize, sqlx::FromRow)]
struct Run {
    id: i32,
}

#[utoipa::path(
    get,
    path = "/features/{feature_id}/runs",
    operation_id = "features-runs-index",
    params(
        ("feature_id" = i32, Path, description = "Feature ID"),
    ),
    responses(
        (status = OK, description = "Run IDs associated with the given feature ID"),
    ),
)]
async fn index(
    Path(feature_id): Path<i32>,
    State(ctx): State<Context>,
) -> server::Result<Json<IndexBody<Vec<Run>>>> {
    let runs = sqlx::query_as!(
        Run,
        "
        select
            runs.id
        from runs
        inner join counts
            on counts.run_id = runs.id
        inner join features
            on features.id = counts.feature_id
        where
            features.id = $1
        ",
        feature_id,
    )
    .fetch_all(&ctx.pool)
    .await?;

    Ok(Json(IndexBody { runs }))
}
