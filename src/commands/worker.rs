use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::{
    cli::WorkerConfig,
    queue::{task::plot, Message, Queue},
};

const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub async fn worker(config: WorkerConfig) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new().connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let queue = Queue::new(pool.clone());

    info!("worker initialized");

    loop {
        if let Some(task) = queue.pull_front().await? {
            info!(id = ?task.id, "received task");

            match task.message.0 {
                Message::Noop => {}
                Message::Plot(configuration_id) => {
                    plot(&pool, configuration_id).await?;
                }
            }

            queue.success(task.id).await?;
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
