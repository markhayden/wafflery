use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use wafflery::host::{api, auth::Auth};

const ORIGIN: &str = "http://127.0.0.1:45678";

fn request(path: &str, method: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .method(method)
        .header("host", "127.0.0.1:45678")
}

#[tokio::test]
async fn browser_boundary_requires_exact_origin_host_and_bearer() {
    let auth = Auth::new().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let temp = tempfile::tempdir().unwrap();
    let store = wafflery::library::assets::AssetStore::open(&temp.path().join("Library")).unwrap();
    let app = api::router(ORIGIN, auth.clone(), quit, store.clone());
    let token = auth.issue().unwrap();
    let body = serde_json::json!({"bootstrap": token}).to_string();
    let exchange = || {
        request("/api/session", "POST")
            .header("origin", ORIGIN)
            .header("content-type", "application/json")
            .body(Body::from(body.clone()))
            .unwrap()
    };
    let response = app.clone().oneshot(exchange()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("set-cookie").is_none());
    assert_eq!(response.headers()["cache-control"], "no-store");
    let session: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(
        app.clone().oneshot(exchange()).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let bearer = format!("Bearer {}", session["token"].as_str().unwrap());
    for (headers, status) in [
        (vec![], StatusCode::UNAUTHORIZED),
        (vec![("cookie", bearer.as_str())], StatusCode::UNAUTHORIZED),
        (vec![("authorization", bearer.as_str())], StatusCode::OK),
        (
            vec![
                ("authorization", bearer.as_str()),
                ("origin", "http://127.0.0.1:45679"),
            ],
            StatusCode::FORBIDDEN,
        ),
        (
            vec![
                ("authorization", bearer.as_str()),
                ("host", "attacker.example"),
            ],
            StatusCode::FORBIDDEN,
        ),
        (
            vec![
                ("authorization", bearer.as_str()),
                ("sec-fetch-site", "cross-site"),
            ],
            StatusCode::FORBIDDEN,
        ),
    ] {
        let mut builder = request("/api/state", "GET");
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        assert_eq!(
            app.clone()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            status
        );
    }
    let no_origin = request("/api/host/quit", "POST")
        .header("authorization", &bearer)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(no_origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let foreign_bootstrap = request("/api/session", "POST")
        .header("origin", "https://example.com")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    assert_eq!(
        app.oneshot(foreign_bootstrap).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

/// A workspace address (the UI's route, `/<tool>/<action>/<run>`) is answered with the page, so
/// a refresh or a shared link lands where it points; a missing file and an unknown API path stay
/// not found.
#[tokio::test]
async fn a_workspace_address_serves_the_page_and_a_missing_file_does_not() {
    let auth = Auth::new().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let temp = tempfile::tempdir().unwrap();
    let store = wafflery::library::assets::AssetStore::open(&temp.path().join("Library")).unwrap();
    let app = api::router(ORIGIN, auth, quit, store);
    let get = |path: &str| request(path, "GET").body(Body::empty()).unwrap();
    let root = to_bytes(
        app.clone().oneshot(get("/")).await.unwrap().into_body(),
        usize::MAX,
    )
    .await
    .unwrap();
    for path in [
        "/rascal",
        "/rascal/remove",
        "/rascal/remove/77448f00-9518-43a9",
    ] {
        let response = app.clone().oneshot(get(path)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html"),
            "{path}"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, root, "{path} is the page");
    }
    for path in ["/assets/missing.js", "/rascal/remove/missing.png"] {
        assert_eq!(
            app.clone().oneshot(get(path)).await.unwrap().status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}
