//! Explicit guided-chain controls. Never accepts commands, paths, replacement data or caps.
use super::run_api::ApiError;
use crate::tools::{chain_replay::ReplayStatus, coordinator::Coordinator};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use std::sync::Arc;
pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/chain-replays", get(list).post(create))
        .route("/api/chain-replays/{id}", get(read).delete(remove))
        .route("/api/chain-replays/{id}/stage", post(stage))
        .route("/api/chain-replays/{id}/stop", post(stop))
        .with_state(coordinator)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    bundle: String,
    tools: Vec<String>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    #[serde(default)]
    resume: bool,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn list(State(c): State<Arc<Coordinator>>) -> Result<Json<Vec<ReplayStatus>>, ApiError> {
    Ok(Json(c.list_chain_replays().await?))
}
async fn read(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<ReplayStatus>, ApiError> {
    Ok(Json(c.chain_replay(&id).await?))
}
async fn create(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Create>,
) -> Result<(StatusCode, Json<ReplayStatus>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            c.start_saved_chain_replay(&request.bundle, request.tools)
                .await?,
        ),
    ))
}
async fn stage(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(request): Json<Stage>,
) -> Result<(StatusCode, Json<ReplayStatus>), ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(c.stage_chain_replay(&id, request.resume).await?),
    ))
}
async fn stop(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<ReplayStatus>, ApiError> {
    Ok(Json(c.stop_chain_replay(&id).await?))
}
async fn remove(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    c.dismiss_chain_replay(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
