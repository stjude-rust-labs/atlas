use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use super::{Context, Error};

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
    use crate::{queue::Message, store::configuration};

    if !dbg!(configuration::exists(&ctx.pool, configuration_id).await?) {
        return Err(Error::NotFound);
    }

    let message = Message::Plot(configuration_id);
    let id = ctx.queue.push_back(message).await?;

    Ok(Json(PlotResponse { id }))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use hyper::{Request, StatusCode};
    use sqlx::PgPool;
    use tower::ServiceExt;

    use super::*;
    use crate::Queue;

    fn app(pool: PgPool) -> Router {
        let queue = Queue::new(pool.clone());
        router().with_state(Context { pool, queue })
    }

    #[sqlx::test]
    async fn test_plot_with_invalid_configuration_id(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/analyses/plot/-1")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
