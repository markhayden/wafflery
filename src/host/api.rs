//! Private browser API: exact origin, explicit bearer credentials, no ambient cookies.
use super::{assets, auth::Auth};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};

#[derive(Clone)]
struct HostState {
    origin: String,
    auth: Auth,
    quit: tokio::sync::watch::Sender<bool>,
    tools: Option<std::sync::Arc<crate::tools::coordinator::Coordinator>>,
}

pub fn router(
    origin: &str,
    auth: Auth,
    quit: tokio::sync::watch::Sender<bool>,
    store: crate::library::assets::AssetStore,
) -> Router {
    router_with_tools(origin, auth, quit, store, None)
}
pub fn router_with_tools(
    origin: &str,
    auth: Auth,
    quit: tokio::sync::watch::Sender<bool>,
    store: crate::library::assets::AssetStore,
    tools: Option<std::sync::Arc<crate::tools::coordinator::Coordinator>>,
) -> Router {
    let routes = if let Some(coordinator) = &tools {
        super::run_api::router(coordinator.clone())
            .merge(super::render_api::router(coordinator.clone(), origin))
            .merge(super::frame_api::router(coordinator.clone(), origin))
    } else {
        Router::new().route(
            "/api/state",
            get(|| async { Json(serde_json::json!({"tools": [], "attempts": []})) }),
        )
    };
    let state = HostState {
        origin: origin.to_owned(),
        auth,
        quit,
        tools,
    };
    Router::new()
        .route("/api/session", post(session))
        .route("/api/host/quit", post(quit_host))
        .route("/api/host/shutdown", get(shutdown_status))
        .route("/api/host/force-quit", post(force_quit))
        .fallback(get(static_asset))
        .with_state(state.clone())
        .merge(routes)
        .merge(super::asset_api::router(store))
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
}

pub(super) fn failure(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({"error": {"code": code, "message": message}})),
    )
        .into_response()
}

fn one_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let first = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(first)
}

async fn boundary(State(state): State<HostState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let mut response = if one_header(headers, "host") != state.origin.strip_prefix("http://")
        || (headers.contains_key("origin")
            && one_header(headers, "origin") != Some(state.origin.as_str()))
        || one_header(headers, "sec-fetch-site") == Some("cross-site")
        || (!matches!(*request.method(), Method::GET | Method::HEAD)
            && one_header(headers, "origin") != Some(state.origin.as_str()))
    {
        failure(
            StatusCode::FORBIDDEN,
            "forbidden_origin",
            "Open this workspace through the Wafflery launcher.",
        )
    } else if request.uri().path().starts_with("/api/")
        && request.uri().path() != "/api/session"
        && !one_header(headers, "authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|token| state.auth.accepts(token))
    {
        failure(
            StatusCode::UNAUTHORIZED,
            "session_required",
            "Run wafflery to open a new workspace session.",
        )
    } else {
        next.run(request).await
    };
    let frame_policy = response
        .extensions_mut()
        .remove::<super::frame_api::FramePolicy>();
    let headers = response.headers_mut();
    for (name, value) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        // Fetch's Origin algorithm can use null for same-origin POST under no-referrer.
        // https://fetch.spec.whatwg.org/#append-a-request-origin-header
        ("referrer-policy", "same-origin"),
        (
            "content-security-policy",
            "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob:; connect-src 'self'; font-src 'self'; object-src 'none'; base-uri 'none'; frame-src 'self'; frame-ancestors 'none'; form-action 'none'",
        ),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    if let Some(policy) = frame_policy {
        headers.insert("content-security-policy", policy.0);
        headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
        headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
        headers.insert(
            "permissions-policy",
            HeaderValue::from_static(
                "camera=(), microphone=(), geolocation=(), display-capture=(), payment=()",
            ),
        );
    }
    response
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Bootstrap {
    bootstrap: String,
}

async fn session(State(state): State<HostState>, Json(input): Json<Bootstrap>) -> Response {
    match state.auth.exchange(&input.bootstrap) {
        Ok(Some(session)) => Json(session).into_response(),
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct QuitRequest {
    #[serde(default)]
    confirmed: bool,
}
async fn quit_host(State(state): State<HostState>, Json(request): Json<QuitRequest>) -> Response {
    if let Some(tools) = &state.tools {
        return match tools.begin_shutdown(request.confirmed, state.quit).await {
            Ok(status) => (StatusCode::ACCEPTED, Json(status)).into_response(),
            Err(error) => super::run_api::ApiError(error).into_response(),
        };
    }
    let _ = state.quit.send(true);
    StatusCode::NO_CONTENT.into_response()
}
async fn shutdown_status(State(state): State<HostState>) -> Response {
    if let Some(tools) = state.tools {
        Json(tools.shutdown_status().await).into_response()
    } else {
        Json(serde_json::json!({"phase":"running","active_attempts":0})).into_response()
    }
}
async fn force_quit(State(state): State<HostState>, Json(_): Json<Empty>) -> Response {
    match state.tools {
        Some(tools) => match tools.force_shutdown(state.quit).await {
            Ok(status) => (StatusCode::ACCEPTED, Json(status)).into_response(),
            Err(error) => super::run_api::ApiError(error).into_response(),
        },
        None => failure(
            StatusCode::CONFLICT,
            "not_waiting",
            "No tool is waiting for force quit.",
        ),
    }
}

async fn static_asset(uri: Uri) -> Response {
    match assets::get(uri.path()) {
        Some(bytes) => {
            let kind = if uri.path().ends_with(".js") {
                "text/javascript"
            } else if uri.path().ends_with(".css") {
                "text/css"
            } else if uri.path().ends_with(".svg") {
                "image/svg+xml"
            } else {
                "text/html; charset=utf-8"
            };
            Response::builder()
                .header("content-type", kind)
                .body(Body::from(bytes))
                .expect("static response")
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
