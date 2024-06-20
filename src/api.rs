use std::{io::Write, str::FromStr, sync::Arc};

use axum::{
    extract::{Multipart, Path, State},
    response::Result,
    routing::{delete, get, put},
    Json, Router,
};
use chrono::Utc;
use image::io::Reader as ImageReader;
use mime_guess::Mime;

use crate::{variant::Original, ImagioError, ImagioImage, ImagioState};

pub async fn list_images_handler(
    State(state): State<Arc<ImagioState>>,
    Path((limit, skip)): Path<(usize, usize)>,
) -> Result<Json<Vec<ImagioImage>>, ImagioError> {
    tracing::info!("Requesting list of images");
    let lock = state.db.read().await;
    let conn = &lock.lock().await;
    let mut stmt = conn
        .prepare(
            "SELECT uuid, category, mime FROM images ORDER BY create_time DESC LIMIT ? OFFSET ? ",
        )
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

pub async fn get_image_handler(
    State(state): State<Arc<ImagioState>>,
    Path(uuid): Path<String>,
) -> Result<Json<ImagioImage>, ImagioError> {
    tracing::info!("Requesting image with uuid: {}", uuid);
    let lock = state.db.read().await;
    let conn = &lock.lock().await;
    let mut stmt = conn
        .prepare("SELECT uuid, category, mime FROM images WHERE uuid = ?")
        .unwrap();
    let mut rows = stmt.query(&[&uuid]).unwrap();

    while let Some(row) = rows.next().unwrap() {
        let uuid: String = row.get(0).unwrap();
        let category: String = row.get(1).unwrap();
        let mime: String = row.get(2).unwrap();
        let image = ImagioImage {
            uuid: uuid.to_string(),
            category,
            mime: Mime::from_str(&mime).unwrap(),
        };
        return Ok(Json(image));
    }
    return Err(ImagioError::NotFound);
}

pub async fn put_image_handler(
    State(state): State<Arc<ImagioState>>,
    mut payload: Multipart,
) -> Result<Json<ImagioImage>, ImagioError> {
    let uuid = uuid::Uuid::new_v4().to_string();

    // Upload the image to local
    while let Ok(Some(field)) = payload.next_field().await {
        let data = field.bytes().await.unwrap().to_vec();
        let image = ImageReader::new(std::io::Cursor::new(data.clone()))
            .with_guessed_format()
            .unwrap();
        let mime_str = image.format().unwrap().to_mime_type();
        let mime = Mime::from_str(&mime_str).unwrap();
        let imagio_image = ImagioImage {
            uuid: uuid.to_string(),
            category: "public".to_string(),
            mime: mime.clone(),
        };

        let image_path = format!(
            "data/images/public/{}.{}",
            uuid,
            mime.subtype().to_string().to_ascii_uppercase()
        );
        let mut file = std::fs::File::create(&image_path).unwrap();
        file.write_all(&data).unwrap();
        tracing::info!("Image saved to: {:?}", image_path);

        let lock = state.db.write().await;
        let conn = &lock.lock().await;
        let mut stmt = conn
            .prepare("INSERT INTO images (uuid, category, mime, create_time) VALUES (?, ?, ?, ?)")
            .unwrap();
        let _ = stmt
            .execute(&[
                &uuid,
                &imagio_image.category,
                mime_str,
                &Utc::now().to_string(),
            ])
            .unwrap();

        return Ok(Json(imagio_image));
    }

    return Err(ImagioError::NotFound);
}

pub async fn delete_image_handler(State(state): State<Arc<ImagioState>>, Path(uuid): Path<String>) {
    let lock = state.db.read().await;
    let conn = &lock.lock().await;
    let mut stmt = conn
        .prepare("SELECT uuid, category, mime FROM images WHERE uuid = ?")
        .unwrap();
    let mut rows = stmt.query(&[&uuid]).unwrap();

    while let Some(row) = rows.next().unwrap() {
        let uuid: String = row.get(0).unwrap();
        let category: String = row.get(1).unwrap();
        let mime: String = row.get(2).unwrap();
        let image = ImagioImage {
            uuid: uuid.to_string(),
            category,
            mime: Mime::from_str(&mime).unwrap(),
        };
        let path = image.original();
        std::fs::remove_file(&path).unwrap();
        tracing::info!("Image deleted from: {:?}", path);
    }

    let mut stmt = conn.prepare("DELETE FROM images WHERE uuid = ?").unwrap();
    let _ = stmt.execute(&[&uuid]).unwrap();
}

pub fn api_router(state: Arc<ImagioState>) -> Router<Arc<ImagioState>> {
    Router::new()
        .route("/images/:limit/:skip", get(list_images_handler))
        .route("/image/:uuid", get(get_image_handler))
        .route("/image/:uuid", delete(delete_image_handler))
        .route("/image", put(put_image_handler))
        .with_state(state)
}
