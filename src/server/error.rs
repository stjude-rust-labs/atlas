use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found")]
    NotFound,
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),
    #[error("internal server error")]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Sqlx(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match &self {
            Self::Sqlx(e) => error!("{e:?}"),
            Self::Anyhow(e) => error!("{e:?}"),
            _ => {}
        }

        (self.status_code(), self.to_string()).into_response()
    }
}
