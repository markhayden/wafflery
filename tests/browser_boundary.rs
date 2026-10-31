use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use wafflehouse::host::{api, auth::Auth};

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
    let app = api::router(ORIGIN, auth.clone(), quit);
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
