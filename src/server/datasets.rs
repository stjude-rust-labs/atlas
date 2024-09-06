use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use super::Context;
use crate::store::dataset::{self, Dataset};

pub fn router() -> Router<Context> {
    Router::new().route("/datasets", get(index))
}

#[derive(Serialize)]
struct IndexResponse {
    datasets: Vec<Dataset>,
}

/// Lists all datasets.
#[utoipa::path(
    get,
    path = "/datasets",
    operation_id = "datasets-index",
    responses(
        (status = OK, description = "A list of datasets"),
    ),
)]
async fn index(State(ctx): State<Context>) -> super::Result<Json<IndexResponse>> {
    let datasets = dataset::all(&ctx.pool).await?;
    Ok(Json(IndexResponse { datasets }))
}
