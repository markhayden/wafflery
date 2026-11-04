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
/// The origin a report server lets frame it: the host's, unless a development page serves the
/// UI from its own loopback origin (Vite at 127.0.0.1:5173 proxying the host). The dev proxy
/// rewrites the host's own policies to that origin, but a tool's report server is framed
/// directly, so it has to be told. Only a loopback `http` origin with a port is accepted.
fn frame_origin(origin: &str) -> String {
    std::env::var("WAFFLERY_DEV_PAGE_ORIGIN")
        .ok()
        .filter(|value| dev_page_origin(value))
        .unwrap_or_else(|| origin.into())
}
fn dev_page_origin(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    // Exactly an origin, as the host passes it on: no path, query or trailing slash.
    url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
        && url.port().is_some()
        && url.origin().ascii_serialization() == value
}
pub(super) fn router(coordinator: Arc<Coordinator>, origin: &str) -> Router {
    let origin = frame_origin(origin);
    Router::new()
        .route("/api/attempts/{id}/views", post(open).get(current))
        .route("/api/views/{id}", get(state))
        .route("/api/views/{id}/activity", post(activity))
        .route("/api/views/{id}/close", post(close))
        .route("/api/views/{id}/force-close", post(force))
        .with_state(Views {
            coordinator,
            origin,
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
async fn activity(
    State(s): State<Views>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(s.coordinator.render_activity(&id).await?))
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

async fn current(State(s): State<Views>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        serde_json::json!({"view": s.coordinator.current_render(&id).await?}),
    ))
}

#[cfg(test)]
mod tests {
    use super::dev_page_origin;
    #[test]
    fn only_a_loopback_http_origin_with_a_port_is_a_dev_page() {
        assert!(dev_page_origin("http://127.0.0.1:5173"));
        assert!(dev_page_origin("http://localhost:5173"));
        for value in [
            "https://127.0.0.1:5173",
            "http://example.com:5173",
            "http://127.0.0.1",
            "http://127.0.0.1:5173/",
            "http://127.0.0.1:5173/x",
            "",
            "not a url",
        ] {
            assert!(!dev_page_origin(value), "{value}");
        }
    }
}
