use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use wafflery::{
    host::{api, auth::Auth},
    library::assets::AssetStore,
};

#[tokio::test]
async fn upload_fetch_and_attachment_enforce_the_asset_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let origin = "http://127.0.0.1:45678";
    let app = api::router(origin, auth, quit, store.clone());
    let request = |path: &str, method: &str| {
        Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", origin)
            .header("authorization", format!("Bearer {}", session.token))
    };
    let image = temp.path().join("example.png");
    image::RgbImage::from_pixel(2, 3, image::Rgb([12_u8, 30, 240]))
        .save(&image)
        .unwrap();
    let bytes = std::fs::read(image).unwrap();
    let upload = request("/api/assets", "POST")
        .header("content-type", "application/octet-stream")
        .header("x-wafflery-upload", "1")
        .body(Body::from(bytes.clone()))
        .unwrap();
    let response = app.clone().oneshot(upload).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let asset: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    for suffix in ["", "/download"] {
        let response = app
            .clone()
            .oneshot(
                request(
                    &format!("/api/assets/{}{suffix}", asset["id"].as_str().unwrap()),
                    "GET",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "image/png");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        if suffix == "/download" {
            assert!(
                response.headers()["content-disposition"]
                    .to_str()
                    .unwrap()
                    .starts_with("attachment;")
            );
        }
        assert_eq!(
            to_bytes(response.into_body(), 4096).await.unwrap().as_ref(),
            bytes
        );
    }
    for (body, expected) in [
        ("<svg onload='alert(1)'/>", StatusCode::UNPROCESSABLE_ENTITY),
        ("<html>bad</html>", StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let response = app
            .clone()
            .oneshot(
                request("/api/assets", "POST")
                    .header("x-wafflery-upload", "1")
                    .header("content-type", "image/png")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    let large = request("/api/assets", "POST")
        .header("x-wafflery-upload", "1")
        .header("content-length", 128 * 1024 * 1024 + 1)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(large).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let unauthenticated = Request::builder()
        .uri("/api/assets")
        .header("host", "127.0.0.1:45678")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(unauthenticated).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let escape = request("/api/assets/%2e%2e%2foutside", "GET")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(escape).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(store.list().await.unwrap().len(), 1);
    let mut oversized_dimensions = bytes.clone();
    oversized_dimensions[16..20].copy_from_slice(&100_000_u32.to_be_bytes());
    oversized_dimensions[20..24].copy_from_slice(&100_000_u32.to_be_bytes());
    let crc = crc32fast::hash(&oversized_dimensions[12..29]);
    oversized_dimensions[29..33].copy_from_slice(&crc.to_be_bytes());
    // The decoder must reject the header without allocating a 10-gigapixel buffer.
    assert!(
        store
            .import_path({
                let path = temp.path().join("huge.png");
                std::fs::write(&path, oversized_dimensions).unwrap();
                path
            })
            .await
            .is_err()
    );
    store.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn upload_size_is_enforced_even_without_a_content_length_header() {
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = api::router("http://127.0.0.1:45678", auth, quit, store.clone());
    let request = Request::builder()
        .method("POST")
        .uri("/api/assets")
        .header("host", "127.0.0.1:45678")
        .header("origin", "http://127.0.0.1:45678")
        .header("authorization", format!("Bearer {}", session.token))
        .header("x-wafflery-upload", "1")
        .body(Body::from(vec![0_u8; 128 * 1024 * 1024 + 1]))
        .unwrap();
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(store.list().await.unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(store.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    store.database.shutdown().await.unwrap();
}
