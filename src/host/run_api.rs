use crate::{
    library::attempts::Submission,
    tools::{Error, coordinator::Coordinator},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::sync::Arc;

pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/state", get(state))
        .route("/api/tools/{id}/description", get(description))
        .route("/api/attempts", get(list).post(submit))
        .route("/api/attempts/{id}", get(attempt))
        .route("/api/attempts/{id}/cancel", post(cancel))
        .route("/api/baselines", get(baseline).post(pin))
        .with_state(coordinator.clone())
        .merge(super::event_api::router(coordinator.events.clone()))
}
pub struct ApiError(pub Error);
impl<E: Into<Error>> From<E> for ApiError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self.0 {
            Error::Conflict(_) | Error::Library(crate::library::Error::Conflict(_)) => {
                (StatusCode::CONFLICT, "conflict")
            }
            Error::Missing(_) | Error::Library(crate::library::Error::Missing(_)) => {
                (StatusCode::NOT_FOUND, "not_found")
            }
            Error::Invalid(_) | Error::Json(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "invalid_inputs")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "execution_unavailable"),
        };
        super::api::failure(status, code, &self.0.to_string())
    }
}
async fn state(State(c): State<Arc<Coordinator>>) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(c.snapshot().await?))
}
async fn description(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<crate::tools::client::Declaration>, ApiError> {
    Ok(Json(c.description(&id).await?))
}
async fn submit(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Submission>,
) -> Result<(StatusCode, Json<crate::library::attempts::Attempt>), ApiError> {
    Ok((StatusCode::ACCEPTED, Json(c.submit(request).await?)))
}
async fn attempt(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<crate::library::attempts::Attempt>, ApiError> {
    Ok(Json(c.attempts.get(&id).await?))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    cursor: Option<i64>,
    limit: Option<usize>,
    tool: Option<String>,
    action: Option<String>,
}
async fn list(
    State(c): State<Arc<Coordinator>>,
    Query(page): Query<Page>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = page.limit.unwrap_or(50).clamp(1, 100);
    let items = c
        .attempts
        .history(page.cursor, limit, page.tool, page.action)
        .await?;
    let cursor = if items.len() == limit {
        items.last().map(|a| a.sequence)
    } else {
        None
    };
    Ok(Json(serde_json::json!({"items":items,"cursor":cursor})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn cancel(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<crate::library::attempts::Attempt>, ApiError> {
    Ok(Json(c.cancel(&id).await?))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    tool: String,
    action: String,
}
async fn baseline(
    State(c): State<Arc<Coordinator>>,
    Query(context): Query<Context>,
) -> Result<Json<Option<String>>, ApiError> {
    Ok(Json(
        crate::library::baselines::BaselineStore(c.assets.database.clone())
            .get(context.tool, context.action)
            .await?,
    ))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    tool: String,
    action: String,
    attempt_id: Option<String>,
}
async fn pin(
    State(c): State<Arc<Coordinator>>,
    Json(pin): Json<Pin>,
) -> Result<StatusCode, ApiError> {
    crate::library::baselines::BaselineStore(c.assets.database.clone())
        .set(pin.tool, pin.action, pin.attempt_id)
        .await?;
    c.events.changed();
    Ok(StatusCode::NO_CONTENT)
}
