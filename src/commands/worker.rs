use serde::Serialize;
use sqlx::postgres::{PgListener, PgPoolOptions};
use tracing::{info, info_span};

use crate::{
    cli::WorkerConfig,
    queue::{task::plot, Message, PlotMessage, Queue},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlotBody {
    sample_names: Vec<String>,
    x: Vec<f64>,
    y: Vec<f64>,
}

pub async fn worker(config: WorkerConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let mut rx = PgListener::connect_with(&pool).await?;
    rx.listen("queue").await?;

    let queue = Queue::new(pool.clone());

    info!("worker initialized");

    loop {
        let _notification = rx.recv().await?;

        if let Some(task) = queue.pull_front().await? {
            let span = info_span!("task", id = ?task.id);
            let _guard = span.enter();

            info!("started processing task");

            match task.message.0 {
                Message::Noop => queue.success(task.id, Option::<()>::None).await?,
                Message::Plot(PlotMessage {
                    configuration_id,
                    additional_runs,
                }) => match plot(&pool, configuration_id, &additional_runs).await {
                    Ok((sample_names, xs, ys)) => {
                        let body = PlotBody {
                            sample_names,
                            x: xs,
                            y: ys,
                        };

                        queue.success(task.id, body).await?;
                    }
                    Err(_) => queue.failed(task.id).await?,
                },
            }

            info!("finished processing task");
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}
