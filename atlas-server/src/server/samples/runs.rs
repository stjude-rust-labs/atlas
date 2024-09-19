use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::{
    server::{Context, Error},
    store::run::Run,
};

pub fn router() -> Router<Context> {
    Router::new().route("/samples/:sample_id/runs", get(index))
}

#[derive(Serialize)]
struct IndexBody {
    runs: Vec<Run>,
}

/// List runs in a sample.
#[utoipa::path(
    get,
    path = "/samples/{sample_id}/runs",
    operation_id = "samples-runs-index",
    params(
        ("sample_id" = i32, Path, description = "Sample ID"),
    ),
    responses(
        (status = OK, description = "Runs associated with the given sample"),
        (status = NOT_FOUND, description = "The sample does not exist"),
    ),
)]
async fn index(
    Path(sample_id): Path<i32>,
    State(ctx): State<Context>,
) -> crate::server::Result<Json<IndexBody>> {
    use crate::store::run;

    let runs = run::where_sample_id(&ctx.pool, sample_id).await?;

    if runs.is_empty() {
        Err(Error::NotFound)
    } else {
        Ok(Json(IndexBody { runs }))
    }
}
