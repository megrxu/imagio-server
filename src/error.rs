use axum::{body::Body, http::StatusCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImagioError {
    #[error("Not found")]
    NotFound,
    #[error("Database Error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("Io Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Mime Guess Error: {0}")]
    MimeError(#[from] mime_guess::mime::FromStrError),
    #[error("Multipart Error: {0}")]
    MultipartError(#[from] axum::extract::multipart::MultipartError),
    #[error("Image Error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Opendal Error: {0}")]
    OpendalError(#[from] opendal::Error),
}

impl axum::response::IntoResponse for ImagioError {
    fn into_response(self) -> axum::http::Response<Body> {
        use ImagioError::*;
        tracing::error!("{:?}", self);
        let (status, body) = match self {
            NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            MultipartError(_) => (StatusCode::BAD_REQUEST, "Bad request".to_string()),
            DatabaseError(_) | IoError(_) | MimeError(_) | ImageError(_) | OpendalError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        axum::http::Response::builder()
            .status(status)
            .body(Body::from(body))
            .unwrap()
    }
}
