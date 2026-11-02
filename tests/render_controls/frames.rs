use super::*;
async fn wrapper(h: &Host, method: &str, path: &str) -> axum::response::Response {
    h.app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "127.0.0.1:4811")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}
#[tokio::test]
async fn private_one_use_frames_have_exact_origins_and_no_api_credentials() {
    let h = Host::new().await;
    for name in ["server", "document"] {
        let opened = h
            .request("POST", "/api/attempts/saved/views", json!({"name":name}))
            .await
            .1;
        let id = opened["id"].as_str().unwrap();
        h.state(id, "ready").await;
        let path = format!("/api/views/{id}/frame");
        assert_eq!(
            h.call("POST", &path, json!({}), "wrong", "http://127.0.0.1:4811")
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            h.request("POST", &path, json!({"url":"https://other.invalid"}))
                .await
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let (status, grant) = h.request("POST", &path, json!({})).await;
        assert_eq!(status, StatusCode::OK);
        let frame = grant["url"].as_str().unwrap();
        assert!(frame.starts_with("/views/"));
        assert!(!frame.contains(&h.token));
        assert_eq!(
            wrapper(&h, "HEAD", frame).await.status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        let response = wrapper(&h, "GET", frame).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert_eq!(response.headers()["x-frame-options"], "SAMEORIGIN");
        let policy = response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .to_owned();
        let body = String::from_utf8(
            to_bytes(response.into_body(), 16384)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        let target = body
            .split("src=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let target = url::Url::parse(target).unwrap();
        assert_ne!(
            target.origin().ascii_serialization(),
            "http://127.0.0.1:4811"
        );
        assert!(policy.contains(&format!(
            "frame-src {};",
            target.origin().ascii_serialization()
        )));
        assert!(policy.contains("frame-ancestors http://127.0.0.1:4811;"));
        assert!(!policy.contains('*'));
        assert!(body.contains("sandbox=\"allow-scripts allow-same-origin\""));
        assert!(!body.contains(&h.token));
        assert!(!body.contains("allow-top-navigation"));
        let replay = wrapper(&h, "GET", frame).await;
        assert_eq!(replay.status(), StatusCode::GONE);
        assert_eq!(replay.headers()["x-frame-options"], "DENY");
        h.request("POST", &format!("/api/views/{id}/close"), json!({}))
            .await;
    }
    let response = wrapper(&h, "GET", "/").await;
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("frame-src 'self'")
    );
    assert_eq!(response.headers()["x-frame-options"], "DENY");
    h.finish().await;
}
#[tokio::test]
async fn grants_cannot_start_unready_views_or_revive_closed_and_replaced_ones() {
    let h = Host::new().await;
    let opened = h
        .request("POST", "/api/attempts/saved/views", json!({"name":"hold"}))
        .await
        .1;
    let id = opened["id"].as_str().unwrap();
    assert_eq!(
        h.request("POST", &format!("/api/views/{id}/frame"), json!({}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    h.request("POST", &format!("/api/views/{id}/close"), json!({}))
        .await;
    let opened = h
        .request(
            "POST",
            "/api/attempts/saved/views",
            json!({"name":"document"}),
        )
        .await
        .1;
    let id = opened["id"].as_str().unwrap();
    h.state(id, "ready").await;
    let grant = h
        .request("POST", &format!("/api/views/{id}/frame"), json!({}))
        .await
        .1;
    h.request("POST", &format!("/api/views/{id}/close"), json!({}))
        .await;
    let frame = grant["url"].as_str().unwrap();
    assert_eq!(
        wrapper(&h, "GET", frame).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(wrapper(&h, "GET", frame).await.status(), StatusCode::GONE);
    assert_eq!(
        h.request("POST", &format!("/api/views/{id}/frame"), json!({}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    h.finish().await;
}
