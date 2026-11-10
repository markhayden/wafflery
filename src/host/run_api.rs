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
        .route("/api/tools/{id}/session", get(session))
        .route("/api/tools/{id}/refresh", post(refresh))
        .route("/api/tools/{id}/restart", post(restart))
        .route(
            "/api/tools/{id}/installables/{installable}/{control}",
            post(installable_control),
        )
        .route("/api/preparations", post(prepare))
        .route(
            "/api/preparations/{id}",
            get(preparation).delete(discard_preparation),
        )
        .route("/api/preparations/{id}/submit", post(submit_prepared))
        .route(
            "/api/preparations/{id}/estimate",
            get(estimate).post(request_estimate),
        )
        .route("/api/attempts", get(list).post(submit))
        .route("/api/attempts/{id}", get(attempt))
        .route("/api/attempts/{id}/cancel", post(cancel))
        .route(
            "/api/attempts/{id}/native-preservation",
            post(preserve_native),
        )
        .route(
            "/api/attempts/{id}/artifacts/{direction}/{index}/export",
            post(export_artifact),
        )
        .route(
            "/api/attempts/{id}/artifacts/{direction}/{index}/import",
            post(import_artifact),
        )
        .route(
            "/api/attempts/{id}/artifacts/{direction}/{index}",
            get(inspect_artifact),
        )
        .route("/api/baselines", get(baseline).post(pin))
        .with_state(coordinator.clone())
        .merge(super::event_api::router(coordinator.events.clone()))
        .merge(super::example_api::router(coordinator.clone()))
        .merge(super::export_api::router(coordinator.clone()))
        .merge(super::chain_replay_api::router(coordinator.clone()))
        .merge(super::bundle_api::router(coordinator.clone()))
        .merge(super::bundle_review_api::router(coordinator.clone()))
        .merge(super::cleanup_api::router(coordinator))
}
pub struct ApiError(pub Error);
impl<E: Into<Error>> From<E> for ApiError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Error::ArtifactControlRefusal(data) = &self.0 {
            return (StatusCode::UNPROCESSABLE_ENTITY,Json(serde_json::json!({"error":{"code":"artifact_refusal","message":data["error"]["reason"],"refusal":data}}))).into_response();
        }
        if let Error::BindingRefusal(data) = &self.0 {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error":{
                    "code":"binding","message":self.0.to_string(),"bindings":data["bindings"]
                }})),
            )
                .into_response();
        }
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
            Error::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "control_timeout"),
            Error::Mcp(rmcp::ServiceError::McpError(error))
                if error.code == rmcp::model::ErrorCode::INVALID_PARAMS =>
            {
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
async fn refresh(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<crate::tools::client::Declaration>, ApiError> {
    Ok(Json(c.refresh(&id).await?))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Restart {
    confirmed: bool,
}
async fn restart(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(input): Json<Restart>,
) -> Result<Json<crate::tools::client::Declaration>, ApiError> {
    Ok(Json(c.restart(&id, input.confirmed).await?))
}
async fn installable_control(
    State(c): State<Arc<Coordinator>>,
    Path((id, installable, control)): Path<(String, String, String)>,
    Json(_): Json<Empty>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    use crate::tools::coordinator::installables::Control;
    let (status, control) = match control.as_str() {
        "install" => (StatusCode::ACCEPTED, Control::Install),
        "uninstall" => (StatusCode::OK, Control::Uninstall),
        _ => return Err(crate::tools::Error::Missing("Unknown installable control").into()),
    };
    Ok((status, Json(c.installable_control(&id, &installable, control).await?)))
}
async fn submit(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Submission>,
) -> Result<(StatusCode, Json<crate::library::attempts::Attempt>), ApiError> {
    Ok((StatusCode::ACCEPTED, Json(c.submit(request).await?)))
}
async fn session(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(c.session(&id).await?))
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

async fn prepare(
    State(c): State<Arc<Coordinator>>,
    Json(request): Json<Submission>,
) -> Result<Json<crate::tools::coordinator::Preparation>, ApiError> {
    Ok(Json(c.prepare(request).await?))
}
async fn preparation(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<crate::tools::coordinator::Preparation>, ApiError> {
    Ok(Json(c.preparation(&id).await?))
}
async fn discard_preparation(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    c.discard_preparation(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn submit_prepared(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(request): Json<Submission>,
) -> Result<(StatusCode, Json<crate::library::attempts::Attempt>), ApiError> {
    if request.id != id {
        return Err(Error::Invalid("Preparation path and submission IDs must match".into()).into());
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(c.submit_prepared(request).await?),
    ))
}

async fn request_estimate(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<crate::tools::coordinator::PreparedEstimate>, ApiError> {
    Ok(Json(c.estimate_prepared(&id).await?))
}
async fn estimate(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
) -> Result<Json<crate::tools::coordinator::PreparedEstimate>, ApiError> {
    Ok(Json(c.prepared_estimate(&id).await?))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactTarget {
    target_tool: Option<String>,
}
async fn preserve_native(
    State(c): State<Arc<Coordinator>>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<crate::library::attempts::Attempt>, ApiError> {
    Ok(Json(c.queue_native_preservation(&id).await?))
}
async fn inspect_artifact(
    State(c): State<Arc<Coordinator>>,
    Path((id, direction, index)): Path<(String, String, usize)>,
    Query(target): Query<ArtifactTarget>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        c.inspect_artifact_at(&id, &direction, index, target.target_tool.as_deref())
            .await?,
    ))
}

async fn export_artifact(
    State(c): State<Arc<Coordinator>>,
    Path((id, direction, index)): Path<(String, String, usize)>,
    Json(_): Json<Empty>,
) -> Result<Json<crate::library::blob_links::BlobLink>, ApiError> {
    Ok(Json(c.export_artifact(&id, &direction, index).await?))
}
async fn import_artifact(
    State(c): State<Arc<Coordinator>>,
    Path((id, direction, index)): Path<(String, String, usize)>,
    Json(target): Json<ArtifactTarget>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        c.import_artifact_into(&id, &direction, index, target.target_tool.as_deref())
            .await?,
    ))
}

#[cfg(test)]
mod binding_error_tests {
    use super::*;
    #[tokio::test]
    async fn estimate_refusal_keeps_indexed_remedies_in_the_http_response() {
        let bindings = serde_json::json!([{"port":"masks","index":2,"problem":"unresolved","expected":"Mask","got":"missing","remedy":"Import the bundle."}]);
        let response = ApiError(Error::BindingRefusal(
            serde_json::json!({"bindings":bindings}),
        ))
        .into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], "binding");
        assert_eq!(body["error"]["bindings"], bindings);
    }
}
