use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{path::Path, process::Command, sync::Arc, time::Duration};
use tower::ServiceExt;
use wafflery::{
    host::{api, auth::Auth},
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};

#[tokio::test]
async fn session_reads_are_authenticated_live_and_do_not_dispatch_or_wait_for_a_run() {
    assert!(
        Command::new(env!("CARGO"))
            .args(["build", "--locked", "--offline", "--example", "sample-lab"])
            .status()
            .unwrap()
            .success()
    );
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let coordinator = Arc::new(
        Coordinator::open(
            assets.clone(),
            vec![Descriptor {
                config: None,
                id: "fixture".into(),
                executable: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("target/debug/examples/sample-lab"),
                args: vec![
                    root.path().join("runs").display().to_string(),
                    "slow".into(),
                ],
                cwd: root.path().into(),
                allow_dirs: vec![root.path().into()],
                evidence_dirs: vec![root.path().into()],
            }],
        )
        .await
        .unwrap(),
    );
    let auth = Auth::new().unwrap();
    let token = auth
        .exchange(&auth.issue().unwrap())
        .unwrap()
        .unwrap()
        .token;
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = api::router_with_tools(
        "http://127.0.0.1:45678",
        auth,
        quit,
        assets.clone(),
        Some(coordinator.clone()),
    );
    let request = |path: &str, authenticated: bool| {
        let mut builder = Request::builder()
            .uri(path)
            .header("host", "127.0.0.1:45678");
        if authenticated {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder.body(Body::empty()).unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(request("/api/tools/fixture/session", false))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.clone()
            .oneshot(request("/api/tools/unknown/session", true))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let response = app
        .clone()
        .oneshot(request("/api/tools/fixture/session", true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let before: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(before["contract"], "0.6.0");
    assert!(before["budget_usd"].is_null());
    assert_eq!(before["spent_usd"], 0);
    assert_eq!(coordinator.snapshot().await.unwrap()["attempts"], json!([]));
    assert!(!root.path().join("runs/calls.ndjson").exists());

    let path = root.path().join("source.png");
    std::fs::write(&path, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(path).await.unwrap();
    let declaration = coordinator.description("fixture").await.unwrap();
    coordinator.submit(serde_json::from_value(json!({"id":"active","tool":"fixture","action":"mark_band","revision":declaration.revision,"params":{"label":"Still running"},"images":{"source":[asset.id]}})).unwrap()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !root.path().join("runs/calls.ndjson").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        app.oneshot(request("/api/tools/fixture/session", true)),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let during: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(during["id"], before["id"]);
    assert!(
        coordinator
            .attempts
            .get("active")
            .await
            .unwrap()
            .result
            .is_none()
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(coordinator.close().await);
    assets.database.shutdown().await.unwrap();
}
