mod counts;
mod error;
mod samples;
pub mod types;

use std::net::SocketAddr;

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
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

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

#[cfg(test)]
pub mod tests {
    use sqlx::{migrate::MigrateDatabase, postgres::PgPoolOptions};

    use super::*;

    pub struct TestPgDatabase {
        database_url: String,
        pub pool: PgPool,
    }

    impl TestPgDatabase {
        pub async fn new(database_url: &str) -> sqlx::Result<TestPgDatabase> {
            let (base_url, _) = database_url.rsplit_once("/").expect("invalid database URL");
            let database_name = generate_name();
            let database_url = format!("{}/{}", base_url, database_name);

            sqlx::Postgres::create_database(&database_url).await?;

            let pool = PgPoolOptions::new().connect(&database_url).await?;
            sqlx::migrate!().run(&pool).await?;

            Ok(Self { database_url, pool })
        }
    }

    impl Drop for TestPgDatabase {
        fn drop(&mut self) {
            let database_url = self.database_url.clone();

            tokio::task::spawn(async move {
                sqlx::Postgres::drop_database(&database_url)
                    .await
                    .expect("could not drop database");
            });
        }
    }

    fn generate_name() -> String {
        use rand::{distributions::Alphanumeric, thread_rng, Rng};

        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    }
}
