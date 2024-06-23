use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    response::Result,
    routing::{delete, get, put},
    Json, Router,
};
use image::io::Reader as ImageReader;

use crate::{variant::Variant, ImagioError, ImagioImage, ImagioState};

async fn list_images_handler(
    State(state): State<Arc<ImagioState>>,
    Path((category, limit, skip)): Path<(String, usize, usize)>,
) -> Result<Json<Vec<ImagioImage>>, ImagioError> {
    tracing::info!("Requesting list of images");
    let images = state.list(category, limit, skip).await?;
    Ok(Json(images))
}

async fn get_image_handler(
    State(state): State<Arc<ImagioState>>,
    Path(uuid): Path<String>,
) -> Result<Json<ImagioImage>, ImagioError> {
    tracing::info!("Requesting image with uuid: {}", uuid);
    let image = state.get(&uuid).await?;
    Ok(Json(image))
}

async fn put_image_handler(
    State(state): State<Arc<ImagioState>>,
    Path(category): Path<String>,
    mut payload: Multipart,
) -> Result<Json<ImagioImage>, ImagioError> {
    let uuid = uuid::Uuid::new_v4().to_string();

    // Upload the image to local
    if let Ok(Some(field)) = payload.next_field().await {
        let data = field.bytes().await?.to_vec();
        let image_blob =
            ImageReader::new(std::io::Cursor::new(data.clone())).with_guessed_format()?;
        let mime_str = image_blob.format().unwrap().to_mime_type();
        let image = ImagioImage::new(&uuid, &category, mime_str)?;

        // Write the image to the store
        image
            .store(
                data,
                state.storage.store.clone(),
                &image.filename(&Variant::Original),
            )
            .await?;

        // Save the image to the database
        state.put(&image).await?;
        tracing::info!("New image uploaded with uuid: {}", uuid);
        return Ok(Json(image));
    }

    Err(ImagioError::NotFound)
}

async fn delete_image_handler(State(state): State<Arc<ImagioState>>, Path(uuid): Path<String>) {
    state.delete(&uuid).await.ok();
}

pub fn api_router(state: Arc<ImagioState>) -> Router<Arc<ImagioState>> {
    Router::new()
        // List images
        .route("/images/:category/:limit/:skip", get(list_images_handler))
        // Get image by uuid
        .route("/image/:uuid", get(get_image_handler))
        // Upload image to category
        .route("/images/:category", put(put_image_handler))
        // Delete image by uuid
        .route("/image/:uuid", delete(delete_image_handler))
        .with_state(state)
        .layer(DefaultBodyLimit::disable())
        .layer(DefaultBodyLimit::max(10 * 1000 * 10))
}
