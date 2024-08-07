mod analyses;
mod configurations;
mod counts;
mod error;
mod runs;
mod samples;
pub mod types;

use axum::{routing::get, Json, Router};
use sqlx::PgPool;
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{services::ServeFile, ServiceBuilderExt};
use tracing::info;
use utoipa::OpenApi;

pub use self::error::Error;
use super::{cli::ServerConfig, store, Queue};

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
        configurations::index,
        configurations::features::index,
        configurations::features::show,
        runs::counts::index,
        samples::index,
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
    samples::router()
        .merge(runs::counts::router())
        .merge(counts::router())
        .merge(configurations::features::router())
        .merge(configurations::router())
        .merge(analyses::plot::router())
        .merge(api_doc_router())
}

pub fn api_doc_router() -> Router<Context> {
    Router::new()
        .route("/openapi.json", get(Json(ApiDoc::openapi())))
        .nest_service("/docs", ServeFile::new("static/docs.html"))
}
