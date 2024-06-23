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
    Ok(Body::from(body))
}

pub async fn server(state: Arc<ImagioState>) -> Result<(), ImagioError> {
    let listener = tokio::net::TcpListener::bind(&state.bind).await?;
    let account_id = state.slug.clone();

    let app = Router::new()
        .route("/:uuid/:variant", get(uuid_handler))
        .nest(
            &format!("/{}", account_id),
            Router::new().nest("/api", api_router(state.clone())),
        )
        .with_state(state);

    axum::serve(listener, app).await?;
    Ok(())
}
