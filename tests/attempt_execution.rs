use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
};
use wafflehouse::{
    library::{assets::AssetStore, attempts::Submission},
    tools::{coordinator::Coordinator, registration::Descriptor},
};
fn binary() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        assert!(
            Command::new(env!("CARGO"))
                .args(["build", "--locked", "--example", "sample-lab"])
                .status()
                .unwrap()
                .success()
        );
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
    })
    .clone()
}
async fn setup(scenario: &str) -> (tempfile::TempDir, Arc<Coordinator>, Submission) {
    let dir = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = dir.path().join("original.png");
    std::fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = store.import_path(input).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            store,
            vec![Descriptor {
                id: "example".into(),
                executable: binary(),
                args: vec![
                    dir.path().join("runs").display().to_string(),
                    scenario.into(),
                ],
                cwd: dir.path().to_owned(),
                roots: vec![dir.path().to_owned()],
            }],
        )
        .await
        .unwrap(),
    );
    let revision = c
        .description("example")
        .await
        .map(|d| d.revision)
        .unwrap_or_default();
    let request=serde_json::from_value(json!({"id":"attempt-1","tool":"example","action":"mark_band","revision":revision,"params":{"label":"original"},"images":{"source":[asset.id]}})).unwrap();
    (dir, c, request)
}
#[tokio::test]
async fn duplicate_submissions_dispatch_once_and_mutable_originals_are_not_inputs() {
    let (dir, c, request) = setup("normal").await;
    let (a, b) = tokio::join!(c.submit(request.clone()), c.submit(request.clone()));
    assert_eq!(a.unwrap().id, b.unwrap().id);
    let mut changed = request.clone();
    changed.params["label"] = json!("changed");
    assert!(c.submit(changed).await.is_err());
    for _ in 0..100 {
        if c.attempts.get("attempt-1").await.unwrap().preservation["contract"] == "complete" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let saved = c.attempts.get("attempt-1").await.unwrap();
    assert_eq!(saved.execution, "completed", "{:?}", saved.notices);
    assert!(saved.wire.is_some());
    assert_eq!(saved.request.params["label"], "original");
    let path = saved.arguments["source"]["path"].as_str().unwrap();
    assert!(!path.contains("assets/sha256"));
    assert_ne!(path, dir.path().join("original.png").to_str().unwrap());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(c.close().await);
    assert!(!std::path::Path::new(path).exists());
    assert!(dir.path().join("original.png").exists());
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn paid_missing_malformed_and_stale_cost_declarations_cannot_dispatch() {
    for scenario in [
        "paid",
        "becomes-paid",
        "missing-cost",
        "malformed-cost",
        "unsupported-contract",
        "normal",
    ] {
        let (dir, c, mut request) = setup(scenario).await;
        if scenario == "normal" {
            request.revision = "stale".into();
        }
        assert!(c.submit(request).await.is_err(), "{scenario}");
        assert!(!dir.path().join("runs/calls.ndjson").exists(), "{scenario}");
        assert!(c.close().await);
        c.assets.database.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn invalid_parameters_and_busy_tool_are_refused() {
    let (dir, c, mut request) = setup("slow").await;
    request.params = json!({"label":"x","unknown":true});
    assert!(c.submit(request.clone()).await.is_err());
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    request.params = json!({"label":"x"});
    c.submit(request.clone()).await.unwrap();
    request.id = "attempt-2".into();
    assert!(c.submit(request).await.is_err());
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn direct_browser_api_cannot_bypass_no_paid_guard() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (dir, c, request) = setup("paid").await;
    let auth = wafflehouse::host::auth::Auth::new().unwrap();
    let token = auth
        .exchange(&auth.issue().unwrap())
        .unwrap()
        .unwrap()
        .token;
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = wafflehouse::host::api::router_with_tools(
        "http://127.0.0.1:9999",
        auth,
        quit,
        c.assets.clone(),
        Some(c.clone()),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attempts")
                .header("host", "127.0.0.1:9999")
                .header("origin", "http://127.0.0.1:9999")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn terminal_evidence_is_verified_and_save_failure_does_not_change_tool_outcome() {
    for scenario in [
        "normal",
        "digest-mismatch",
        "missing-output",
        "conflicting-run",
    ] {
        let (_dir, c, request) = setup(scenario).await;
        c.submit(request).await.unwrap();
        let mut saved = c.attempts.get("attempt-1").await.unwrap();
        for _ in 0..100 {
            saved = c.attempts.get("attempt-1").await.unwrap();
            if saved.preservation["contract"] != "pending" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        if scenario == "conflicting-run" {
            assert_eq!(saved.execution, "interrupted");
        } else {
            assert_eq!(saved.execution, "completed", "{:?}", saved.notices);
            assert_eq!(saved.result.as_ref().unwrap()["status"], "completed");
            assert_eq!(
                saved.preservation["contract"],
                if scenario == "normal" {
                    "complete"
                } else {
                    "incomplete"
                }
            );
            assert_eq!(saved.preservation["native"], "unsupported");
            if scenario == "normal" {
                assert_eq!(saved.outputs.len(), 1);
                assert!(saved.journal.is_some());
            } else {
                assert!(saved.outputs.is_empty());
                assert!(!saved.notices.is_empty());
            }
        }
        assert!(saved.raw_response.as_ref().unwrap()["structuredContent"].is_object());
        assert!(c.close().await);
        c.assets.database.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn cancellation_without_a_run_reference_stays_unconfirmed_and_reserved() {
    let (dir, c, request) = setup("slow").await;
    c.submit(request.clone()).await.unwrap();
    for _ in 0..100 {
        if c.attempts.get(&request.id).await.unwrap().wire.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let first = c.cancel(&request.id).await.unwrap();
    let second = c.cancel(&request.id).await.unwrap();
    assert_eq!(first.execution, "unconfirmed");
    assert_eq!(second.execution, "unconfirmed");
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let saved = c.attempts.get(&request.id).await.unwrap();
    assert_eq!(saved.execution, "unconfirmed", "{:?}", saved.notices);
    assert!(saved.result.is_none());
    let mut another = request;
    another.id = "attempt-2".into();
    assert!(c.submit(another).await.is_err());
    assert!(!c.try_quit().await);
    assert!(c.cleanup(Default::default(), String::new()).await.is_err());
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("runs/run-0001/run.json")).unwrap())
            .unwrap();
    assert_eq!(result["status"], "cancelled");
    assert!(c.close().await);
    assert_eq!(
        c.attempts.get("attempt-1").await.unwrap().execution,
        "interrupted"
    );
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn impossible_image_coordinate_metadata_is_rejected_before_dispatch() {
    let (dir, c, mut request) = setup("normal").await;
    request.frames = serde_json::from_value(
        json!({"source":[{"projection":"equirectangular","topology":"bounded"}]}),
    )
    .unwrap();
    assert!(c.submit(request).await.is_err());
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn refreshed_declarations_replace_the_browser_snapshot_without_dispatch() {
    let (dir, c, request) = setup("becomes-paid").await;
    let old = request.revision.clone();
    assert!(c.submit(request).await.is_err());
    let current = c.description("example").await.unwrap();
    assert_ne!(current.revision, old);
    assert_eq!(current.document["actions"][0]["cost"]["may_spend"], true);
    assert_eq!(
        c.snapshot().await.unwrap()["tools"][0]["declaration"]["revision"],
        current.revision
    );
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn child_exit_releases_the_reservation_without_claiming_a_tool_outcome() {
    let (_dir, c, request) = setup("exit-during-call").await;
    c.submit(request).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(4), async {
        loop {
            if c.attempts.get("attempt-1").await.unwrap().execution == "interrupted"
                && c.try_quit().await
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("dead child must not block quit");
    assert!(c.attempts.get("attempt-1").await.unwrap().result.is_none());
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_cancelled_child_that_exits_later_does_not_hold_the_reservation() {
    let (dir, c, request) = setup("exit-after-cancel").await;
    c.submit(request).await.unwrap();
    for _ in 0..100 {
        if dir.path().join("runs/calls.ndjson").exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    c.cancel("attempt-1").await.unwrap();
    assert!(!c.try_quit().await);
    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    assert!(
        !c.snapshot().await.unwrap()["tools"][0]["busy"]
            .as_bool()
            .unwrap()
    );
    assert!(c.try_quit().await);
    let saved = c.attempts.get("attempt-1").await.unwrap();
    assert_eq!(saved.execution, "interrupted");
    assert!(saved.result.is_none());
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn active_shutdown_requires_confirmation_and_keeps_unknown_outcomes_honest() {
    let (_dir, c, request) = setup("slow").await;
    c.submit(request).await.unwrap();
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    assert!(c.begin_shutdown(false, quit.clone()).await.is_err());
    assert_eq!(c.shutdown_status().await.phase, "running");
    c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(8),
        stopped.wait_for(|value| *value),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(c.shutdown_status().await.phase, "stopped");
    let attempt = c.attempts.get("attempt-1").await.unwrap();
    assert!(matches!(
        attempt.execution.as_str(),
        "interrupted" | "rejected"
    ));
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn an_unresponsive_owned_child_needs_a_separate_force_request() {
    let (_dir, c, _request) = setup("ignore-eof").await;
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    assert!(c.force_shutdown(quit.clone()).await.is_err());
    c.begin_shutdown(false, quit.clone()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        while c.shutdown_status().await.phase != "waiting" {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(!*stopped.borrow());
    c.force_shutdown(quit).await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(8),
        stopped.wait_for(|value| *value),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(c.shutdown_status().await.phase, "stopped");
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_waits_for_terminal_resource_preservation_before_closing_the_peer() {
    let (_dir, c, request) = setup("slow-resource").await;
    c.submit(request).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let attempt = c.attempts.get("attempt-1").await.unwrap();
            if attempt.result.is_some() && attempt.preservation["contract"] == "pending" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(8),
        stopped.wait_for(|value| *value),
    )
    .await
    .unwrap()
    .unwrap();
    let attempt = c.attempts.get("attempt-1").await.unwrap();
    assert_eq!(
        attempt.preservation["contract"], "complete",
        "{:?}",
        attempt.notices
    );
    assert!(!c.assets.root().join("execution/attempt-1").exists());
    c.assets.database.shutdown().await.unwrap();
}
