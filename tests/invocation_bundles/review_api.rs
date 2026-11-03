use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use wafflery::{
    host::{api, auth::Auth},
    library::bundle_store::BundleStore,
    tools::coordinator::Coordinator,
};
#[tokio::test]
async fn upload_review_is_bounded_private_and_has_no_import_publication_or_run_effects() {
    let (_source_root, source, bytes) = super::reader::exported().await;
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let coordinator = Arc::new(Coordinator::open(assets.clone(), vec![]).await.unwrap());
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = api::router_with_tools(
        "http://127.0.0.1:45678",
        auth,
        quit,
        assets.clone(),
        Some(coordinator.clone()),
    );
    let request = |method: &str, path: &str, bytes: Vec<u8>, authorized: bool, mime: &str| {
        let mut r = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("content-type", mime);
        if authorized {
            r = r.header("authorization", format!("Bearer {}", session.token));
        }
        r.body(Body::from(bytes)).unwrap()
    };
    let denied = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/bundle-reviews",
            bytes.clone(),
            false,
            "application/x-tar",
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let mut oversized = request(
        "POST",
        "/api/bundle-reviews",
        vec![],
        true,
        "application/x-tar",
    );
    oversized
        .headers_mut()
        .insert("content-length", "268435457".parse().unwrap());
    assert_eq!(
        app.clone().oneshot(oversized).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let malformed = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/bundle-reviews",
            b"not an archive".to_vec(),
            true,
            "application/x-tar",
        ))
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let wrong = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/bundle-reviews",
            bytes.clone(),
            true,
            "application/json",
        ))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // A raw Body upload must not inherit the JSON extractor's 4 MiB limit.
    let mut entries = vec![];
    for entry in tar::Archive::new(bytes.as_slice()).entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        let mut body = vec![];
        entry.read_to_end(&mut body).unwrap();
        entries.push((name, body));
    }
    let mut manifest: Value = serde_json::from_slice(&entries[0].1).unwrap();
    let native = manifest["steps"][0]["artifacts"][0]["member"]
        .as_str()
        .unwrap()
        .to_owned();
    let data = vec![b'n'; 5 * 1024 * 1024];
    let sha = hash(&data);
    for artifact in manifest["steps"][0]["artifacts"].as_array_mut().unwrap() {
        artifact["sha256"] = json!(sha);
    }
    entries[0].1 = serde_json::to_vec(&manifest).unwrap();
    entries
        .iter_mut()
        .find(|(name, _)| name == &native)
        .unwrap()
        .1 = data;
    let mut archive = tar::Builder::new(vec![]);
    for (name, data) in entries {
        let mut h = tar::Header::new_ustar();
        h.set_size(data.len() as u64);
        h.set_uid(0);
        h.set_gid(0);
        h.set_mode(0o644);
        h.set_mtime(0);
        archive.append_data(&mut h, name, data.as_slice()).unwrap();
    }
    let large = archive.into_inner().unwrap();
    assert!(large.len() > 4 * 1024 * 1024);
    let mut first = String::new();
    for payload in [large, bytes.clone()] {
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                "/api/bundle-reviews",
                payload.clone(),
                true,
                "application/x-tar",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(value["record"]["id"], hash(&payload));
        assert!(value["import"].is_null());
        first = value["id"].as_str().unwrap().into();
    }
    let overflow = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/bundle-reviews",
            bytes.clone(),
            true,
            "application/x-tar",
        ))
        .await
        .unwrap();
    assert_eq!(overflow.status(), StatusCode::CONFLICT);
    let import = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/bundle-reviews/{first}/import"),
            br#"{"step":0,"tool":"not-installed"}"#.to_vec(),
            true,
            "application/json",
        ))
        .await
        .unwrap();
    assert_eq!(import.status(), StatusCode::NOT_FOUND);
    assert!(
        BundleStore(assets.clone())
            .list(None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        coordinator
            .attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read_dir(assets.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    let response = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/bundle-reviews/{first}"),
            vec![],
            true,
            "application/json",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let removed = app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/bundle-reviews/{first}"),
            vec![],
            true,
            "application/json",
        ))
        .await
        .unwrap();
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);
    let third = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/bundle-reviews",
            bytes,
            true,
            "application/x-tar",
        ))
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::CREATED);
    assert!(coordinator.close().await);
    source.database.shutdown().await.unwrap();
    assets.database.shutdown().await.unwrap();
}
