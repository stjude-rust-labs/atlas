use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug)]
pub enum Error {
    NotFound,
    Sqlx(sqlx::Error),
    Anyhow(anyhow::Error),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Sqlx(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("not found"),
            Self::Sqlx(_) => f.write_str("database error"),
            Self::Anyhow(_) => f.write_str("internal server error"),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Sqlx(ref e) => {
                error!("{:?}", e);
            }
            Self::Anyhow(ref e) => {
                error!("{:?}", e);
            }
            _ => {}
        }

        (self.status_code(), self.to_string()).into_response()
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}
