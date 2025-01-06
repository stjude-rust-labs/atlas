use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::{
    server::{Context, Error},
    store::run::{self, Run},
};

pub fn router() -> Router<Context> {
    Router::new().route("/datasets/{dataset_id}/runs", get(index))
}

#[derive(Serialize)]
struct IndexBody {
    runs: Vec<Run>,
}

/// List run in a dataset.
#[utoipa::path(
    get,
    path = "/datasets/{dataset_id}/runs",
    operation_id = "datasets-runs-index",
    params(
        ("dataset_id" = i32, Path, description = "Dataset ID"),
    ),
    responses(
        (status = OK, description = "Runs associated with the given dataset"),
        (status = NOT_FOUND, description = "The dataset does not exist"),
    ),
)]
async fn index(
    State(ctx): State<Context>,
    Path(dataset_id): Path<i32>,
) -> crate::server::Result<Json<IndexBody>> {
    let runs = run::where_dataset_id(&ctx.pool, dataset_id).await?;

    if runs.is_empty() {
        Err(Error::NotFound)
    } else {
        Ok(Json(IndexBody { runs }))
    }
}
