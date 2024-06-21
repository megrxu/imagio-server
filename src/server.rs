use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    routing::get,
    Router,
};

use crate::{api::*, variant::Variant, ImagioError, ImagioState};

pub async fn uuid_handler(
    Path((uuid, variant)): Path<(String, Variant)>,
    State(state): State<Arc<ImagioState>>,
) -> axum::response::Result<Body, ImagioError> {
    tracing::info!("Requesting image with uuid: {}", uuid);
    let image = state.get(&uuid).await?;
    let body = state.variant(&image, variant).await?;
    return Ok(Body::from(body));
}

pub async fn server(state: Arc<ImagioState>) -> Result<(), ImagioError> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000").await?;
    let account_id = state.slug.clone();

    let app = Router::new().nest(
        &format!("/{}", account_id),
        Router::new()
            .nest("/api", api_router(state.clone()))
            .route("/:uuid/:variant", get(uuid_handler))
            .with_state(state),
    );

    axum::serve(listener, app).await?;
    Ok(())
}
