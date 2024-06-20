use std::{str::FromStr, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, State},
    routing::get,
    Router,
};
use mime_guess::Mime;

use crate::{
    variant::{ImageVariant, Variant},
    ImagioError, ImagioImage, ImagioState,
};

pub async fn uuid_handler(
    Path((uuid, variant)): Path<(String, Variant)>,
    State(state): State<Arc<ImagioState>>,
) -> axum::response::Result<Body, ImagioError> {
    tracing::info!("Requesting image with uuid: {}", uuid);
    let lock = state.db.read().await;
    let conn = &lock.lock().await;
    let mut stmt = conn
        .prepare("SELECT category, mime FROM images WHERE uuid = ?")
        .unwrap();
    let mut rows = stmt.query(&[&uuid.to_string()]).unwrap();

    let row = rows.next().unwrap();
    if let Some(row) = row {
        let category: String = row.get(0).unwrap();
        let mime: String = row.get(1).unwrap();
        let image = ImagioImage {
            uuid: uuid.to_string(),
            category,
            mime: Mime::from_str(&mime).unwrap(),
        };
        let body = image.variant(variant)?;
        return Ok(body);
    } else {
        return Err(ImagioError::NotFound);
    }
}

pub async fn server(state: Arc<ImagioState>) -> Result<(), ImagioError> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000").await?;

    let token = state.token.clone();
    let app = Router::new().nest(
        &format!("/{}", token),
        Router::new()
            .route("/:uuid/:variant", get(uuid_handler))
            .with_state(state),
    );

    axum::serve(listener, app).await?;
    Ok(())
}
