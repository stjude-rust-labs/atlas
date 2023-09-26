use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, info_span};

use crate::{
    cli::WorkerConfig,
    queue::{task::plot, Message, Queue},
};

#[derive(Serialize)]
struct PlotBody {
    x: Vec<f32>,
    y: Vec<f32>,
}

pub async fn worker(config: WorkerConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let queue = Queue::new(pool.clone());

    info!("worker initialized");

    loop {
        if let Some(task) = queue.pull_front().await? {
            let span = info_span!("task", id = ?task.id);
            let _guard = span.enter();

            info!("started processing task");

            match task.message.0 {
                Message::Noop => queue.success(task.id, Option::<()>::None).await?,
                Message::Plot(configuration_id) => match plot(&pool, configuration_id).await {
                    Ok((xs, ys)) => {
                        let body = PlotBody { x: xs, y: ys };
                        queue.success(task.id, body).await?;
                    }
                    Err(_) => queue.failed(task.id).await?,
                },
            }

            info!("finished processing task");
        }

        tokio::time::sleep(config.poll_interval).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
