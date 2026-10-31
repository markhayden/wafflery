use super::run_api::ApiError;
use crate::{
    library::cleanup::{Cleanup, Outcome, Preview, Selection},
    tools::coordinator::Coordinator,
};
use axum::{Json, Router, extract::State, routing::post};
use std::sync::Arc;
pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/cleanup/preview", post(preview))
        .route("/api/cleanup/confirm", post(confirm))
        .with_state(coordinator)
}
async fn preview(
    State(c): State<Arc<Coordinator>>,
    Json(selection): Json<Selection>,
) -> Result<Json<Preview>, ApiError> {
    Ok(Json(Cleanup(c.assets.clone()).preview(selection).await?))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmation {
    selection: Selection,
    token: String,
}
async fn confirm(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Confirmation>,
) -> Result<Json<Outcome>, ApiError> {
    Ok(Json(c.cleanup(request.selection, request.token).await?))
}
