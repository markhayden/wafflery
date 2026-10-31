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
}

pub fn router(origin: &str, auth: Auth, quit: tokio::sync::watch::Sender<bool>) -> Router {
    let state = HostState {
        origin: origin.to_owned(),
        auth,
        quit,
    };
    Router::new()
        .route("/api/session", post(session))
        .route(
            "/api/state",
            get(|| async { Json(serde_json::json!({"tools": [], "attempts": []})) }),
        )
        .route("/api/host/quit", post(quit_host))
        .fallback(get(static_asset))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state)
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
        StatusCode::FORBIDDEN.into_response()
    } else if request.uri().path().starts_with("/api/")
        && request.uri().path() != "/api/session"
        && !one_header(headers, "authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|token| state.auth.accepts(token))
    {
        StatusCode::UNAUTHORIZED.into_response()
    } else {
        next.run(request).await
    };
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
            "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob:; connect-src 'self'; font-src 'self'; object-src 'none'; base-uri 'none'; frame-src 'none'; frame-ancestors 'none'; form-action 'none'",
        ),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
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

async fn quit_host(State(state): State<HostState>, Json(_): Json<Empty>) -> StatusCode {
    let _ = state.quit.send(true);
    StatusCode::NO_CONTENT
}

async fn static_asset(uri: Uri) -> Response {
    match assets::get(uri.path()) {
        Some(bytes) => {
            let kind = if uri.path().ends_with(".js") {
                "text/javascript"
            } else if uri.path().ends_with(".css") {
                "text/css"
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
