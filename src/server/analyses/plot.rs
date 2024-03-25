use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    queue,
    server::{self, Context, Error},
};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/analyses/plot", post(create))
        .route("/analyses/plot/:id", get(show))
}

#[derive(Deserialize)]
pub(crate) struct CreateRequestRun {
    sample_name: String,
    counts: HashMap<String, i32>,
}

#[derive(Deserialize, ToSchema)]
struct CreateRequest {
    configuration_id: i32,
    additional_runs: Option<Vec<CreateRequestRun>>,
}

#[derive(Serialize)]
struct CreateResponse {
    id: Uuid,
}

/// Submits a task to perform dimension reduction on all samples in a configuation.
#[utoipa::path(
    post,
    path = "/analyses/plot",
    request_body = inline(CreateRequest),
    responses(
        (status = OK, description = "The ID of the task submitted"),
        (status = NOT_FOUND, description = "The configuration ID does not exist"),
    ),
)]
async fn create(
    State(ctx): State<Context>,
    Json(body): Json<CreateRequest>,
) -> server::Result<Json<CreateResponse>> {
    use crate::{queue::Message, store::configuration};

    let CreateRequest {
        configuration_id,
        additional_runs,
    } = body;

    if !configuration::exists(&ctx.pool, configuration_id).await? {
        return Err(Error::NotFound);
    }

    let additional_runs = additional_runs
        .map(|runs| {
            runs.into_iter()
                .map(|run| (run.sample_name, run.counts))
                .collect()
        })
        .unwrap_or_default();

    let message = Message::Plot(configuration_id, additional_runs);
    let id = ctx.queue.push_back(message).await?;

    Ok(Json(CreateResponse { id }))
}

#[derive(Deserialize, Serialize)]
struct Body {
    x: Vec<f32>,
    y: Vec<f32>,
}

#[derive(Serialize)]
struct Task {
    id: Uuid,
    status: queue::Status,
    body: Option<sqlx::types::Json<Body>>,
}

/// Returns the status of a plot task.
#[utoipa::path(
    get,
    path = "/analyses/plot/{id}",
    params(
        ("id" = Uuid, Path, description = "Task ID"),
    ),
    responses(
        (status = OK, description = "Plot task status"),
        (status = INTERNAL_SERVER_ERROR, description = "The task ID does not exist"),
    ),
)]
async fn show(State(ctx): State<Context>, Path(task_id): Path<Uuid>) -> server::Result<Json<Task>> {
    let task = sqlx::query_as!(
        Task,
        r#"
        select
            tasks.id,
            status as "status: queue::Status",
            results.body as "body: Option<sqlx::types::Json<Body>>"
        from tasks
        left join results
            on tasks.id  = results.id
        where tasks.id = $1
        "#,
        task_id
    )
    .fetch_one(&ctx.pool)
    .await?;

    Ok(Json(task))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use hyper::{header, Request, StatusCode};
    use serde_json::json;
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
        let payload = json!({ "configuration_id": -1 });
        let body = Body::from(payload.to_string());
        let request = Request::post("/analyses/plot")
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)?;

        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[sqlx::test]
    async fn test_show_with_invalid_task_id(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::get("/analyses/plot/5970136c-f1bf-405a-aa79-a81595101864")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        Ok(())
    }
}
