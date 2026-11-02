//! Trusted wrapper markup only. Tool HTML remains on its own exact origin.
use super::{api::failure, frame_grants::FrameGrants, run_api::ApiError};
use crate::tools::{Error, coordinator::Coordinator};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Clone)]
struct Frames {
    coordinator: Arc<Coordinator>,
    origin: String,
    grants: FrameGrants,
}
/// Only this validated handler can request the wrapper-specific boundary headers.
#[derive(Clone)]
pub(super) struct FramePolicy(pub HeaderValue);
pub(super) fn router(coordinator: Arc<Coordinator>, origin: &str) -> Router {
    Router::new()
        .route("/api/views/{id}/frame", post(issue))
        .route("/views/{token}", get(wrapper))
        .with_state(Frames {
            coordinator,
            origin: origin.into(),
            grants: FrameGrants::default(),
        })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn issue(
    State(state): State<Frames>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Json<Value>, ApiError> {
    state.coordinator.frame_target(&id, &state.origin).await?;
    let token = state.grants.issue(&id).map_err(Error::from)?;
    Ok(Json(json!({"url":format!("/views/{token}")})))
}
async fn wrapper(
    State(state): State<Frames>,
    Path(token): Path<String>,
    method: Method,
    uri: Uri,
) -> Result<Response, ApiError> {
    if method != Method::GET {
        return Ok(StatusCode::METHOD_NOT_ALLOWED.into_response());
    }
    if uri.query().is_some() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    let Some(id) = state.grants.consume(&token) else {
        return Ok(failure(
            StatusCode::GONE,
            "expired_frame",
            "This report link expired. Open the report again.",
        ));
    };
    let target = state.coordinator.frame_target(&id, &state.origin).await?;
    let policy=HeaderValue::from_str(&format!("default-src 'none'; style-src 'unsafe-inline'; frame-src {}; frame-ancestors {}; base-uri 'none'; form-action 'none'",target.origin().ascii_serialization(),state.origin))
        .map_err(|_|Error::Invalid("Invalid report frame policy".into()))?;
    let target = escape_attribute(target.as_str());
    let mut response=Html(format!("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Report viewer</title><style>html,body,iframe{{width:100%;height:100%;margin:0;border:0}}body{{overflow:hidden}}</style></head><body><iframe title=\"Tool report\" src=\"{target}\" sandbox=\"allow-scripts allow-same-origin\" referrerpolicy=\"no-referrer\"></iframe></body></html>")).into_response();
    response.extensions_mut().insert(FramePolicy(policy));
    Ok(response)
}
fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&#39;")
}
