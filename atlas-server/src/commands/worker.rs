use serde::Serialize;
use sqlx::postgres::{PgListener, PgPoolOptions};
use tokio::{signal, sync::watch};
use tracing::{info, info_span};

use crate::{
    cli::WorkerConfig,
    queue::{Message, PlotMessage, Queue, task::plot},
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

    let (signal_tx, signal_rx) = watch::channel(());

    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C listener");

        info!("received shutdown signal");

        drop(signal_rx);
    });

    info!("worker initialized");

    let mut wait_for_notification = false;

    loop {
        if wait_for_notification {
            tokio::select! {
                _notification = rx.recv() => {},
                _ = signal_tx.closed() => break,
            }
        } else if signal_tx.is_closed() {
            break;
        }

        if let Some(task) = queue.pull_front().await? {
            let span = info_span!("task", id = ?task.id);
            let _guard = span.enter();

            info!("started processing task");

            match task.message.0 {
                Message::Noop => queue.success(task.id, Option::<()>::None).await?,
                Message::Plot(PlotMessage {
                    configuration_id,
                    additional_runs,
                    options,
                }) => match plot(&pool, configuration_id, &additional_runs, options).await {
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
        } else {
            wait_for_notification = true;
        }
    }

    Ok(())
}
