use std::fmt::Write;

use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};

use super::Context;

pub fn router() -> Router<Context> {
    Router::new().route("/analyses/plot/:configuration_id", get(plot))
}

/// Performs dimension reduction on all samples in a configuation.
#[utoipa::path(
    get,
    path = "/analyses/plot/{configuration-id}",
    params(
        ("configuration-id" = i32, Path, description = "Configuration ID"),
    ),
    responses(
        (status = OK, description = "Coordinates in two dimensions"),
        (status = NOT_FOUND, description = "The configuration ID does not exist"),
    ),
)]
async fn plot(
    State(ctx): State<Context>,
    Path(configuration_id): Path<i32>,
) -> super::Result<String> {
    use crate::queue::task;

    let embedding = task::plot(&ctx.pool, configuration_id).await?;

    let mut body = String::new();

    for p in embedding.chunks_exact(2) {
        let (x, y) = (p[0], p[1]);
        writeln!(body, "{x},{y}").map_err(anyhow::Error::new)?;
    }

    Ok(body)
}
