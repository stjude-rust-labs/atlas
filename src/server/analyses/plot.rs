use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
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
struct CreateRequest {
    configuration_id: i32,
}

#[derive(Serialize)]
struct CreateResponse {
    id: Uuid,
}

/// Submits a task to perform dimension reduction on all samples in a configuation.
#[utoipa::path(
    post,
    path = "/analyses/plot",
    params(
        ("configuration-id" = i32, Path, description = "Configuration ID"),
    ),
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

    let configuration_id = body.configuration_id;

    if !dbg!(configuration::exists(&ctx.pool, configuration_id).await?) {
        return Err(Error::NotFound);
    }

    let message = Message::Plot(configuration_id);
    let id = ctx.queue.push_back(message).await?;

    Ok(Json(CreateResponse { id }))
}

#[derive(Serialize)]
struct Task {
    id: Uuid,
    status: queue::Status,
    body: Option<sqlx::types::Json<Vec<f32>>>,
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
            results.body as "body: Option<sqlx::types::Json<Vec<f32>>>"
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
    use hyper::{header, Body, Request, StatusCode};
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
        let body = Body::from(r#"{"configuration_id":-1}"#);
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
