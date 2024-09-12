mod create;

use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
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
    store::feature::find_features,
};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/analyses/plot", post(create))
        .route("/analyses/plot/:id", get(show))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CreateRequest {
    dataset_id: i32,
    additional_runs: Option<HashMap<String, HashMap<String, i32>>>,
    #[schema(inline)]
    options: Option<create::Options>,
}

#[derive(Serialize)]
struct CreateResponse {
    id: Uuid,
}

/// Submits a task to perform dimension reduction on all samples in a configuation.
#[utoipa::path(
    post,
    path = "/analyses/plot",
    operation_id = "analyses-plot-create",
    request_body = inline(CreateRequest),
    responses(
        (status = OK, description = "The ID of the task submitted"),
        (status = NOT_FOUND, description = "The dataset ID does not exist"),
        (status = INTERNAL_SERVER_ERROR, description = "The additional runs input is invalid"),
    ),
)]
async fn create(
    State(ctx): State<Context>,
    Json(body): Json<CreateRequest>,
) -> server::Result<Json<CreateResponse>> {
    use self::create::{merge_options, validate_run};
    use crate::{
        queue::{Message, PlotMessage},
        store::dataset,
    };

    let CreateRequest {
        dataset_id,
        additional_runs,
        options,
    } = body;

    if !dataset::exists(&ctx.pool, dataset_id).await? {
        return Err(Error::NotFound);
    }

    let additional_runs: Vec<_> = additional_runs
        .map(|runs| runs.into_iter().collect())
        .unwrap_or_default();

    let configuration_ids = dataset::configuration_ids(&ctx.pool, dataset_id).await?;

    if configuration_ids.len() != 1 {
        return Err(Error::Anyhow(anyhow!("dataset is nonhomogeneous")));
    }

    // SAFETY: `configuration_ids` is non-empty.
    let configuration_id = configuration_ids[0];

    let features = find_features(&ctx.pool, configuration_id).await?;
    let feature_names: HashSet<_> = features.into_iter().map(|(_, name)| name).collect();

    for (_, run) in &additional_runs {
        validate_run(&feature_names, run).map_err(anyhow::Error::new)?;
    }

    let mut message_options = crate::queue::task::plot::Options::default();

    if let Some(arguments) = options {
        merge_options(&mut message_options, &arguments);
    }

    let message = Message::Plot(PlotMessage {
        configuration_id,
        additional_runs,
        options: message_options,
    });

    let id = ctx.queue.push_back(message).await?;

    Ok(Json(CreateResponse { id }))
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    sample_names: Vec<String>,
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
    operation_id = "analyses-plot-show",
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
    async fn test_plot_with_invalid_dataset_id(pool: PgPool) -> anyhow::Result<()> {
        let payload = json!({ "datasetId": -1 });
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
