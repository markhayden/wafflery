use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Duration,
};
use wafflery::{
    library::{assets::AssetStore, attempts::Submission},
    tools::{coordinator::Coordinator, registration::Descriptor},
};

async fn setup(label: &str) -> (tempfile::TempDir, Arc<Coordinator>, Submission) {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    let binary = BINARY.get_or_init(|| {
        assert!(
            std::process::Command::new(env!("CARGO"))
                .args(["build", "--locked", "--example", "sample-lab"])
                .status()
                .unwrap()
                .success()
        );
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
    });
    let dir = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = dir.path().join("original.png");
    std::fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(input).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            assets,
            vec![Descriptor {
                id: "fixture".into(),
                config: None,
                executable: binary.clone(),
                args: vec![
                    dir.path().join("runs").display().to_string(),
                    "estimates".into(),
                ],
                cwd: dir.path().to_owned(),
                roots: vec![dir.path().to_owned()],
            }],
        )
        .await
        .unwrap(),
    );
    let declaration = c.description("fixture").await.unwrap();
    let request = serde_json::from_value(json!({"id":"prepared","tool":"fixture","action":"price_test","revision":declaration.revision,"params":{"label":label,"max_spend_usd":0.5},"images":{"source":[asset.id]}})).unwrap();
    (dir, c, request)
}
async fn marker(path: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("peer did not reach the expected gate");
}
async fn finish(c: &Arc<Coordinator>) {
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), stopped.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn estimates_use_exact_prepared_inputs_while_a_free_run_holds_execution() {
    let (dir, c, request) = setup("normal").await;
    let mut free = request.clone();
    free.id = "running".into();
    free.action = "mark_band".into();
    free.params.as_object_mut().unwrap().remove("max_spend_usd");
    c.submit(free).await.unwrap();
    marker(&dir.path().join("runs/calls.ndjson")).await;
    c.prepare(request.clone()).await.unwrap();
    std::fs::write(
        dir.path().join("original.png"),
        b"changed after preparation",
    )
    .unwrap();
    let quote = c.estimate_prepared(&request.id).await;
    let snapshot = c.snapshot().await.unwrap();
    let recorded: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("runs/estimate-started")).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("runs/release-run"), b"").unwrap();
    finish(&c).await;
    assert!(quote.unwrap().estimate["allowed"].as_bool().unwrap());
    assert_eq!(snapshot["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["tools"][0]["busy"], true);
    assert_eq!(
        recorded["source"]["expect"]["file_sha256"],
        std::fs::read_to_string(dir.path().join("runs/estimate-read")).unwrap()
    );
    assert!(
        recorded["source"]["path"]
            .as_str()
            .unwrap()
            .contains("execution/prepared/")
    );
}
#[tokio::test]
async fn abandoned_request_and_discard_keep_remote_input_lease_and_bound_concurrency() {
    let (dir, c, request) = setup("hold").await;
    c.prepare(request.clone()).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    assert!(c.estimate_prepared("prepared").await.is_err());
    task.abort();
    c.discard_preparation("prepared").await.unwrap();
    let path = c.assets.root().join("execution/prepared");
    assert!(path.exists());
    std::fs::write(dir.path().join("runs/release-estimate"), b"").unwrap();
    marker(&dir.path().join("runs/estimate-read")).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(c.preparation("prepared").await.is_err());
    assert!(
        c.snapshot().await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/estimates.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    finish(&c).await;
}
#[tokio::test]
async fn timed_out_estimate_keeps_reader_alive_without_retries_or_late_quote() {
    let (dir, c, request) = setup("hold").await;
    c.prepare(request).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    let result = tokio::time::timeout(Duration::from_secs(35), task)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    assert!(
        c.session("fixture").await.is_ok(),
        "timeout releases ledger observation lock"
    );
    assert!(
        c.estimate_prepared("prepared").await.is_err(),
        "old remote reader still owns the estimate slot"
    );
    c.discard_preparation("prepared").await.unwrap();
    let path = c.assets.root().join("execution/prepared");
    assert!(path.exists());
    std::fs::write(dir.path().join("runs/release-estimate"), b"").unwrap();
    marker(&dir.path().join("runs/estimate-read")).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(c.preparation("prepared").await.is_err());
    assert!(
        c.snapshot().await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    finish(&c).await;
}
#[tokio::test]
async fn malformed_quotes_and_rpc_refusals_create_no_attempt() {
    for label in ["wrong-session", "plain-error", "structured-error"] {
        let (_dir, c, request) = setup(label).await;
        c.prepare(request).await.unwrap();
        assert!(c.estimate_prepared("prepared").await.is_err(), "{label}");
        assert!(
            c.snapshot().await.unwrap()["attempts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        finish(&c).await;
    }
}

#[tokio::test]
async fn shutdown_closes_the_reader_before_joining_estimate_workers() {
    let (dir, c, request) = setup("hold").await;
    c.prepare(request).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    let path = c.assets.root().join("execution/prepared");
    assert!(path.exists());
    finish(&c).await;
    assert!(task.await.unwrap().is_err());
    assert!(!path.exists());
    assert!(!dir.path().join("runs/estimate-read").exists());
}

#[tokio::test]
async fn broken_transport_keeps_inputs_until_the_owned_child_is_confirmed_stopped() {
    let (dir, c, request) = setup("disconnect").await;
    std::fs::write(dir.path().join("runs/hold-after-eof"), b"").unwrap();
    c.prepare(request).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    c.discard_preparation("prepared").await.unwrap();
    let path = c.assets.root().join("execution/prepared");
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        while c.shutdown_status().await.phase != "waiting" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        path.exists(),
        "transport loss and EOF are not confirmation of process exit"
    );
    c.force_shutdown(quit).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), stopped.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(!path.exists());
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn estimate_api_is_private_strict_and_never_accepts_caller_bindings() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let (dir, c, input) = setup("normal").await;
    c.prepare(input).await.unwrap();
    let auth = wafflery::host::auth::Auth::new().unwrap();
    let token = auth
        .exchange(&auth.issue().unwrap())
        .unwrap()
        .unwrap()
        .token;
    let (quit, _) = tokio::sync::watch::channel(false);
    let origin = "http://127.0.0.1:9999";
    let app = wafflery::host::api::router_with_tools(
        origin,
        auth,
        quit,
        c.assets.clone(),
        Some(c.clone()),
    );
    let request = |method: &str, credential: &str, site: &str, body: Value| {
        Request::builder()
            .method(method)
            .uri("/api/preparations/prepared/estimate")
            .header("host", "127.0.0.1:9999")
            .header("origin", site)
            .header("authorization", format!("Bearer {credential}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    for (credential, site, expected) in [
        ("wrong", origin, StatusCode::UNAUTHORIZED),
        (&token[..], "https://other.invalid", StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request("POST", credential, site, json!({})))
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(request("GET", &token, origin, json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    for body in [
        json!({"binding":"invented"}),
        json!({"max_spend_usd":9}),
        json!({"arguments":{}}),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request("POST", &token, origin, body))
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert!(!dir.path().join("runs/estimates.ndjson").exists());
    let response = app
        .clone()
        .oneshot(request("POST", &token, origin, json!({})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["preparation"]["id"], "prepared");
    assert_eq!(body["estimate"]["allowed"], true);
    assert!(body["estimate"].get("binding").is_none());
    assert_eq!(body["estimate"]["effective"].as_object().unwrap().len(), 2);
    assert!(!body.to_string().contains("execution/"));
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(request("GET", &token, origin, json!({})))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let stored: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(stored, body);
    }
    assert!(
        c.snapshot().await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    c.discard_preparation("prepared").await.unwrap();
    assert_eq!(
        app.oneshot(request("GET", &token, origin, json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    finish(&c).await;
}

#[tokio::test]
async fn quote_observations_update_snapshot_balance_without_resetting_prepared_inputs() {
    let (_dir, c, input) = setup("ledger").await;
    let before = c.prepare(input.clone()).await.unwrap();
    c.estimate_prepared("prepared").await.unwrap();
    let snapshot = c.snapshot().await.unwrap();
    assert_eq!(
        snapshot["tools"][0]["declaration"]["tool_session"]["spent_usd"],
        json!(0.08)
    );
    assert_eq!(
        snapshot["tools"][0]["declaration"]["revision"],
        before.revision
    );
    assert_eq!(
        c.preparation("prepared").await.unwrap().expires_ms,
        before.expires_ms
    );
    assert!(snapshot["attempts"].as_array().unwrap().is_empty());
    finish(&c).await;
}

#[tokio::test]
async fn readiness_refusal_remains_an_unknown_cost_observation_without_an_attempt() {
    let (_dir, c, input) = setup("readiness").await;
    c.prepare(input).await.unwrap();
    let view = c.estimate_prepared("prepared").await.unwrap();
    assert_eq!(view.estimate["allowed"], false);
    assert_eq!(view.estimate["refused"], "readiness");
    assert!(view.estimate["worst_case_usd"].is_null());
    assert!(view.estimate["effective"]["config_identity"].is_null());
    assert_eq!(
        c.prepared_estimate("prepared").await.unwrap().estimate,
        view.estimate
    );
    assert!(
        c.snapshot().await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    finish(&c).await;
}

#[tokio::test]
async fn snapshot_keeps_latest_balance_while_an_estimate_holds_the_ledger_lock() {
    let (dir, c, input) = setup("ledger").await;
    c.prepare(input.clone()).await.unwrap();
    let mut held = input;
    held.id = "held-estimate".into();
    held.params["label"] = json!("hold");
    let prepared = c.prepare(held).await.unwrap();
    c.estimate_prepared("prepared").await.unwrap();
    assert_eq!(
        c.snapshot().await.unwrap()["tools"][0]["declaration"]["tool_session"]["spent_usd"],
        json!(0.08)
    );
    std::fs::remove_file(dir.path().join("runs/estimate-started")).unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("held-estimate").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    let snapshot = c.snapshot().await.unwrap();
    std::fs::write(dir.path().join("runs/release-estimate"), b"").unwrap();
    task.await.unwrap().unwrap();
    finish(&c).await;
    assert_eq!(
        snapshot["tools"][0]["declaration"]["tool_session"]["spent_usd"],
        json!(0.08)
    );
    assert_eq!(
        snapshot["tools"][0]["declaration"]["revision"],
        prepared.revision
    );
}
