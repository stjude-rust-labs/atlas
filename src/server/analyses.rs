use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use super::Context;

pub fn router() -> Router<Context> {
    Router::new().route("/analyses/plot/:configuration_id", get(plot))
}

#[derive(Serialize)]
struct PlotResponse {
    id: Uuid,
}

/// Submits a task to perform dimension reduction on all samples in a configuation.
#[utoipa::path(
    get,
    path = "/analyses/plot/{configuration-id}",
    params(
        ("configuration-id" = i32, Path, description = "Configuration ID"),
    ),
    responses(
        (status = OK, description = "The ID of the task submitted"),
        (status = NOT_FOUND, description = "The configuration ID does not exist"),
    ),
)]
async fn plot(
    State(ctx): State<Context>,
    Path(configuration_id): Path<i32>,
) -> super::Result<Json<PlotResponse>> {
    use crate::queue::Message;

    let message = Message::Plot(configuration_id);
    let id = ctx.queue.push_back(message).await?;

    Ok(Json(PlotResponse { id }))
}
