pub mod counts;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::Serialize;

use super::{Context, Error};
use crate::store::run::{self, Run};

pub fn router() -> Router<Context> {
    Router::new().route("/runs/{id}", get(show))
}

#[derive(Serialize)]
struct ShowResponse {
    run: Run,
}

/// Finds a run by ID.
#[utoipa::path(
    get,
    path = "/runs/{id}",
    operation_id = "runs-show",
    params(
        ("id" = i32, Path, description = "Run ID"),
    ),
    responses(
        (status = OK, description = "The run of the given ID"),
        (status = NOT_FOUND, description = "The run does not exist"),
    ),
)]
async fn show(
    State(ctx): State<Context>,
    Path(id): Path<i32>,
) -> super::Result<Json<ShowResponse>> {
    let run = run::find(&ctx.pool, id).await?.ok_or(Error::NotFound)?;
    Ok(Json(ShowResponse { run }))
}
