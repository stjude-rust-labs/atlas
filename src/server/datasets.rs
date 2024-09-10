pub mod runs;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use super::Context;
use crate::{
    server::Error,
    store::dataset::{self, Dataset},
};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/datasets", get(index))
        .route("/datasets/:id", get(show))
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

#[derive(Serialize)]
struct ShowResponse {
    dataset: Dataset,
}

/// Find a sample by ID.
#[utoipa::path(
    get,
    path = "/datasets/{id}",
    operation_id = "datasets-show",
    params(
        ("id" = i32, Path, description = "Dataset ID"),
    ),
    responses(
        (status = OK, description = "The dataset of the given ID"),
        (status = NOT_FOUND, description = "The dataset does not exist"),
    ),
)]
async fn show(
    State(ctx): State<Context>,
    Path(id): Path<i32>,
) -> super::Result<Json<ShowResponse>> {
    let dataset = dataset::find(&ctx.pool, id).await?.ok_or(Error::NotFound)?;
    Ok(Json(ShowResponse { dataset }))
}
