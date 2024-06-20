use axum::{body::Body, http::StatusCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImagioError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("Not found")]
    NotFound,
    #[error("Internal server error: {0}")]
    InternalServerError(#[from] std::io::Error),
}

impl axum::response::IntoResponse for ImagioError {
    fn into_response(self) -> axum::http::Response<Body> {
        let (status, body) = match self {
            ImagioError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            ImagioError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            ImagioError::InvalidVariant(_) => (StatusCode::BAD_REQUEST, "Invalid variant"),
            ImagioError::InternalServerError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };
        axum::http::Response::builder()
            .status(status)
            .body(Body::from(body))
            .unwrap()
    }
}
