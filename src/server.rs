mod counts;
mod error;
mod samples;
pub mod types;

use std::net::{Ipv4Addr, SocketAddr};

use axum::{routing::get, Json, Router};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tracing::info;
use utoipa::OpenApi;

pub use self::error::Error;
use super::cli::ServerConfig;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(OpenApi)]
#[openapi(paths(counts::show, samples::index, samples::show))]
struct ApiDoc;

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
}

pub async fn serve(config: &ServerConfig, pool: PgPool) -> anyhow::Result<()> {
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));

    let service = ServiceBuilder::new().trace_for_http();
    let ctx = Context { pool };
    let app = router().layer(service).with_state(ctx);

    info!("listening on {addr}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

fn router() -> Router<Context> {
    samples::router()
        .merge(counts::router())
        .merge(api_doc_router())
}

pub fn api_doc_router() -> Router<Context> {
    Router::new().route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
}
