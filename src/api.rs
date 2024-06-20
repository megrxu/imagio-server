use std::{str::FromStr, sync::Arc};

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use mime_guess::Mime;

use crate::{ImagioError, ImagioImage, ImagioState};

pub async fn list_images_handler(
    State(state): State<Arc<ImagioState>>,
    Path((limit, skip)): Path<(usize, usize)>,
) -> axum::response::Result<Json<Vec<ImagioImage>>, ImagioError> {
    tracing::info!("Requesting list of images");
    let lock = state.db.read().await;
    let conn = &lock.lock().await;
    let mut stmt = conn
        .prepare("SELECT uuid, category, mime FROM images LIMIT ? OFFSET ?")
        .unwrap();
    let mut rows = stmt.query(&[&(limit as i64), &(skip as i64)]).unwrap();

    let mut images = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        let uuid: String = row.get(0).unwrap();
        let category: String = row.get(1).unwrap();
        let mime: String = row.get(2).unwrap();
        let image = ImagioImage {
            uuid: uuid.to_string(),
            category,
            mime: Mime::from_str(&mime).unwrap(),
        };
        images.push(image);
    }

    Ok(Json(images))
}

pub fn api_router(state: Arc<ImagioState>) -> Router<Arc<ImagioState>> {
    Router::new()
        .route("/images/:limit/:skip", get(list_images_handler))
        .with_state(state)
}
