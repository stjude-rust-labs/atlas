mod counts;
mod error;
mod samples;
pub mod types;

use std::net::{Ipv4Addr, SocketAddr};

use axum::Router;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tracing::info;

pub use self::error::Error;
use super::cli::ServerConfig;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
}

pub async fn serve(config: &ServerConfig, pool: PgPool) -> anyhow::Result<()> {
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));

    let service = ServiceBuilder::new()
        .add_extension(Context { pool })
        .trace_for_http();

    let app = router().layer(service);

    info!("listening on {addr}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

fn router() -> Router {
    samples::router().merge(counts::router())
}
