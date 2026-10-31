use super::run_api::ApiError;
use crate::{
    library::examples::{Example, ExampleStore},
    tools::coordinator::Coordinator,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
pub fn router(c: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/examples", get(list).post(save))
        .route("/api/examples/{id}", get(open))
        .with_state(c)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    cursor: Option<i64>,
}
async fn list(
    State(c): State<Arc<Coordinator>>,
    Query(page): Query<Page>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let items = ExampleStore(c.assets.database.clone())
        .list(page.cursor)
        .await?;
    let cursor = if items.len() == 20 {
        items.last().map(|item| item.sequence)
    } else {
        None
    };
    Ok(Json(serde_json::json!({"items":items,"cursor":cursor})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    attempt_id: String,
    name: String,
    notes: String,
}
async fn save(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Save>,
) -> Result<Json<Example>, ApiError> {
    let saved = ExampleStore(c.assets.database.clone())
        .save(request.attempt_id, request.name, request.notes)
        .await?;
    c.events.changed();
    Ok(Json(saved))
}
async fn open(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<Example>, ApiError> {
    Ok(Json(ExampleStore(c.assets.database.clone()).get(id).await?))
}
