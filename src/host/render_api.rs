use super::run_api::ApiError;
use crate::tools::coordinator::Coordinator;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
#[derive(Clone)]
struct Views {
    coordinator: Arc<Coordinator>,
    origin: String,
}
pub(super) fn router(coordinator: Arc<Coordinator>, origin: &str) -> Router {
    Router::new()
        .route("/api/attempts/{id}/views", post(open))
        .route("/api/views/{id}", get(state))
        .route("/api/views/{id}/close", post(close))
        .route("/api/views/{id}/force-close", post(force))
        .with_state(Views {
            coordinator,
            origin: origin.into(),
        })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Open {
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Force {
    confirmed: bool,
}
async fn open(
    State(s): State<Views>,
    Path(id): Path<String>,
    Json(body): Json<Open>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        s.coordinator
            .open_render(&id, &body.name, &s.origin)
            .await?,
    ))
}
async fn state(State(s): State<Views>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(s.coordinator.render_state(&id).await?))
}
async fn close(
    State(s): State<Views>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(s.coordinator.close_render(&id, false, false).await?))
}
async fn force(
    State(s): State<Views>,
    Path(id): Path<String>,
    Json(body): Json<Force>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        s.coordinator
            .close_render(&id, true, body.confirmed)
            .await?,
    ))
}
