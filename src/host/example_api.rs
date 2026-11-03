use super::run_api::ApiError;
use crate::{
    library::examples::{Example, ExampleStore},
    tools::coordinator::Coordinator,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;
pub fn router(c: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/examples", get(list).post(save))
        .route("/api/examples/{id}", get(open))
        .route(
            "/api/examples/{id}/artifacts/{port}/{index}",
            get(inspect_artifact),
        )
        .route(
            "/api/examples/{id}/artifacts/{port}/{index}/import",
            post(import_artifact),
        )
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
    #[serde(default)]
    allow_workspace_dependent: bool,
}
async fn save(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Save>,
) -> Result<Json<Example>, ApiError> {
    let _storage = c.assets.storage.read().await;
    let attempt = c.attempts.get(&request.attempt_id).await?;
    for (_, _, id) in attempt.request.bound_images() {
        c.assets.read(id).await?;
    }
    let blobs = crate::library::blobs::BlobStore(c.assets.clone());
    let mut verified = std::collections::BTreeSet::new();
    for link in blobs
        .links(crate::library::blob_links::Owner::Attempt(
            request.attempt_id.clone(),
        ))
        .await?
        .into_iter()
        .filter(|link| link.direction == "in")
    {
        if verified.insert(link.sha256.clone()) {
            blobs.read_leased(&link.sha256).await?;
        }
    }
    let saved = ExampleStore(c.assets.database.clone())
        .save_with_options(
            request.attempt_id,
            request.name,
            request.notes,
            request.allow_workspace_dependent,
        )
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    target_tool: String,
}
async fn inspect_artifact(
    State(c): State<Arc<Coordinator>>,
    Path((id, port, index)): Path<(String, String, usize)>,
    Query(target): Query<Target>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        c.inspect_example_artifact(&id, &port, index, &target.target_tool)
            .await?,
    ))
}
async fn import_artifact(
    State(c): State<Arc<Coordinator>>,
    Path((id, port, index)): Path<(String, String, usize)>,
    Json(target): Json<Target>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        c.import_example_artifact(&id, &port, index, &target.target_tool)
            .await?,
    ))
}
