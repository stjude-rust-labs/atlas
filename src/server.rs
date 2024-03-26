mod analyses;
mod configurations;
mod counts;
mod error;
mod samples;
pub mod types;

use std::net::{Ipv4Addr, SocketAddr};

use axum::{routing::get, Json, Router};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{services::ServeFile, ServiceBuilderExt};
use tracing::info;
use utoipa::OpenApi;

pub use self::error::Error;
use super::{cli::ServerConfig, Queue};

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
        counts::show,
        samples::index,
        samples::show,
    )
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

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));
    let listener = TcpListener::bind(addr).await?;

    info!("listening on {addr}");

    axum::serve(listener, app).await?;

    Ok(())
}

fn router() -> Router<Context> {
    samples::router()
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
