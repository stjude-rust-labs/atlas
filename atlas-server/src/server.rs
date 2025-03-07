mod analyses;
mod configurations;
mod counts;
mod datasets;
mod error;
mod features;
mod runs;
mod samples;

use axum::{Json, Router, routing::get};
use sqlx::PgPool;
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{ServiceBuilderExt, services::ServeFile};
use tracing::info;
use utoipa::OpenApi;

pub use self::error::Error;
use super::{Queue, cli::ServerConfig, store};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(OpenApi)]
#[openapi(
    servers((
        url = "http://localhost:{port}",
        description = "Local development server",
        variables(
            ("port" = (default = "3000", description = "Port")),
        ),
    )),
    paths(
        analyses::plot::create,
        analyses::plot::show,
        configurations::features::index,
        configurations::features::show,
        configurations::index,
        configurations::show,
        counts::index,
        datasets::index,
        datasets::runs::index,
        datasets::show,
        features::runs::index,
        runs::show,
        runs::counts::index,
        samples::index,
        samples::runs::index,
        samples::show,
    ),
    components(schemas(store::StrandSpecification)),
)]
struct ApiDoc;

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    queue: Queue,
}

pub async fn serve(config: &ServerConfig, pool: PgPool) -> anyhow::Result<()> {
    let service = ServiceBuilder::new().trace_for_http();

    let queue = Queue::new(pool.clone());
    let ctx = Context { pool, queue };

    let app = router().layer(service).with_state(ctx);

    let addr = &config.bind;
    let listener = TcpListener::bind(addr).await?;

    info!("listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C listener");

            info!("received shutdown signal");
        })
        .await?;

    Ok(())
}

fn router() -> Router<Context> {
    samples::runs::router()
        .merge(samples::router())
        .merge(runs::counts::router())
        .merge(runs::router())
        .merge(features::runs::router())
        .merge(datasets::runs::router())
        .merge(datasets::router())
        .merge(counts::router())
        .merge(configurations::features::router())
        .merge(configurations::router())
        .merge(analyses::plot::router())
        .merge(api_doc_router())
}

pub fn api_doc_router() -> Router<Context> {
    Router::new()
        .route("/openapi.json", get(Json(ApiDoc::openapi())))
        .nest_service("/docs", ServeFile::new("atlas-server/static/docs.html"))
}
