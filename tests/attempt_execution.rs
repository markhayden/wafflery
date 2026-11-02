use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
};
use wafflery::{
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
                config: None,
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
// close() is one bounded shutdown step and may defer during evidence preservation.
// Ordinary teardown uses the same observable completion signal as the host, without forcing.
async fn finish_host(c: &Arc<Coordinator>) {
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(8),
        stopped.wait_for(|value| *value),
    )
    .await
    .expect("graceful host shutdown did not complete")
    .unwrap();
    assert_eq!(c.shutdown_status().await.phase, "stopped");
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
    finish_host(&c).await;
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
        finish_host(&c).await;
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
    finish_host(&c).await;
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
    let auth = wafflery::host::auth::Auth::new().unwrap();
    let token = auth
        .exchange(&auth.issue().unwrap())
        .unwrap()
        .unwrap()
        .token;
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = wafflery::host::api::router_with_tools(
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
    finish_host(&c).await;
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
        finish_host(&c).await;
        c.assets.database.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn cancellation_without_a_run_reference_stays_unconfirmed_and_reserved() {
    let (dir, c, request) = setup("unassociated").await;
    c.submit(request.clone()).await.unwrap();
    // This case tests cancellation after allocation. Wire persistence alone precedes allocation.
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !dir.path().join("runs/calls.ndjson").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let first = c.cancel(&request.id).await.unwrap();
    let second = c.cancel(&request.id).await.unwrap();
    assert_eq!(first.execution, "unconfirmed");
    assert_eq!(second.execution, "unconfirmed");
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let Ok(bytes) = std::fs::read(dir.path().join("runs/run-0001/run.json"))
                && let Ok(result) = serde_json::from_slice::<serde_json::Value>(&bytes)
                && result["status"] == "cancelled"
            {
                break result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let saved = c.attempts.get(&request.id).await.unwrap();
    assert_eq!(saved.execution, "unconfirmed", "{:?}", saved.notices);
    assert!(saved.result.is_none());
    let mut another = request;
    another.id = "attempt-2".into();
    assert!(c.submit(another).await.is_err());
    assert!(!c.try_quit().await);
    assert!(c.cleanup(Default::default(), String::new()).await.is_err());
    assert_eq!(result["status"], "cancelled");
    finish_host(&c).await;
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
    finish_host(&c).await;
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
    finish_host(&c).await;
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
    finish_host(&c).await;
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
    if saved.execution == "cancelled" {
        assert_eq!(saved.result.as_ref().unwrap()["status"], "cancelled");
    } else {
        assert_eq!(saved.execution, "interrupted");
        assert!(saved.result.is_none());
    }
    finish_host(&c).await;
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
    let (dir, c, _request) = setup("ignore-eof").await;
    let snapshot = c.snapshot().await.unwrap();
    assert!(snapshot["tools"][0]["declaration"].is_null());
    assert!(
        snapshot["tools"][0]["error"]
            .as_str()
            .unwrap()
            .contains("Configuration probe did not stop")
    );
    assert!(!dir.path().join("runs/calls.ndjson").exists());
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

#[tokio::test]
async fn cancelled_runs_are_recovered_from_progress_or_the_runs_list() {
    for scenario in ["slow", "no-progress", "paged"] {
        let (dir, c, request) = setup(scenario).await;
        c.submit(request.clone()).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while !dir.path().join("runs/calls.ndjson").exists() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        c.cancel(&request.id).await.unwrap();
        let saved = tokio::time::timeout(std::time::Duration::from_secs(8), async {
            loop {
                let saved = c.attempts.get(&request.id).await.unwrap();
                if saved.preservation["contract"] == "incomplete"
                    && saved.observations.get("session_after").is_some()
                {
                    break saved;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
        let saved = match saved {
            Ok(saved) => saved,
            Err(error) => panic!(
                "{scenario}: {error}; saved={:?}",
                c.attempts.get(&request.id).await.unwrap()
            ),
        };
        assert_eq!(saved.execution, "cancelled");
        assert!(
            saved
                .notices
                .iter()
                .any(|notice| notice.contains("input evidence is incomplete"))
        );
        assert!(
            saved.raw_response.is_none(),
            "resource recovery is not a tool-call response"
        );
        assert_eq!(
            saved.observations["reconciled_from"],
            saved.result.as_ref().unwrap()["run"]["resource"]
        );
        assert!(saved.journal.is_some());
        assert_eq!(
            saved.observations["session_after"]["id"],
            saved.declaration["tool_session"]["id"]
        );
        assert_eq!(
            saved.wire.as_ref().unwrap()["params"]["_meta"]["table-manners/client_run_id"],
            request.id
        );
        let mut next = request;
        next.id = "next".into();
        // Persistence finishes before the reservation is released.
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while c.snapshot().await.unwrap()["tools"][0]["busy"] == true {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        c.submit(next).await.unwrap();
        finish_host(&c).await;
        c.assets.database.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn mismatched_correlation_and_session_never_attach_a_result() {
    for scenario in ["wrong-correlation", "wrong-session"] {
        let (_dir, c, request) = setup(scenario).await;
        c.submit(request.clone()).await.unwrap();
        let saved = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let saved = c.attempts.get(&request.id).await.unwrap();
                if saved.preservation["contract"] != "pending" {
                    break saved;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(saved.result.is_none());
        assert!(!saved.notices.is_empty());
        assert_eq!(c.snapshot().await.unwrap()["tools"][0]["busy"], true);
        finish_host(&c).await;
        c.assets.database.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn cancellation_conflicts_or_incomplete_pagination_never_release_the_reservation() {
    for scenario in ["duplicate-pages", "conflict-once", "endless-pages"] {
        let (dir, c, request) = setup(scenario).await;
        c.submit(request.clone()).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while !dir.path().join("runs/calls.ndjson").exists() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        c.cancel(&request.id).await.unwrap();
        let expected_notice = match scenario {
            "duplicate-pages" => "Multiple runs claim",
            "conflict-once" => "Runs list belongs to a different tool",
            _ => "pagination exceeded",
        };
        let saved = tokio::time::timeout(std::time::Duration::from_secs(6), async {
            loop {
                let saved = c.attempts.get(&request.id).await.unwrap();
                if saved
                    .notices
                    .iter()
                    .any(|notice| notice.contains(expected_notice))
                {
                    break saved;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            saved.execution, "unconfirmed",
            "{scenario}: {:?}",
            saved.notices
        );
        assert!(saved.result.is_none());
        assert_eq!(c.snapshot().await.unwrap()["tools"][0]["busy"], true);
        finish_host(&c).await;
        c.assets.database.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn a_reset_session_cannot_silently_dispatch_with_a_fresh_ledger() {
    let (dir, c, request) = setup("session-reset").await;
    let error = c.submit(request).await.unwrap_err().to_string();
    assert!(error.contains("session changed"), "{error}");
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    finish_host(&c).await;
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn rpc_errors_with_owned_run_references_reconcile_without_another_dispatch() {
    for scenario in ["known-run-rpc-error", "reused-run", "reused-foreign-run"] {
        let (dir, c, request) = setup(scenario).await;
        c.submit(request.clone()).await.unwrap();
        let saved = tokio::time::timeout(std::time::Duration::from_secs(6), async {
            loop {
                let attempt = c.attempts.get(&request.id).await.unwrap();
                if attempt.preservation["contract"] != "pending" {
                    break attempt;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let duplicate = c.submit(request).await.unwrap();
        let count = std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count();
        finish_host(&c).await;
        c.assets.database.shutdown().await.unwrap();
        if scenario == "reused-foreign-run" {
            assert_eq!(saved.execution, "interrupted");
            assert!(saved.result.is_none());
            assert!(saved.observations.get("spent_usd").is_none());
        } else {
            assert_eq!(
                saved.execution, "completed",
                "{scenario}: {:?}",
                saved.notices
            );
            assert_eq!(saved.preservation["contract"], "complete");
            assert_eq!(
                saved.observations["reconciled_from"],
                saved.result.as_ref().unwrap()["run"]["resource"]
            );
        }
        assert!(saved.raw_response.as_ref().unwrap()["error"].is_object());
        assert_eq!(duplicate.id, saved.id);
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn null_claims_queued_cancellation_and_disconnect_never_become_no_run_refusals() {
    for scenario in [
        "reused-null",
        "queued",
        "queued-resource-error",
        "exit-during-call",
    ] {
        let (dir, c, request) = setup(scenario).await;
        c.submit(request.clone()).await.unwrap();
        if scenario.starts_with("queued") {
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                while !dir.path().join("runs/queued").exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            c.cancel(&request.id).await.unwrap();
        }
        let saved = tokio::time::timeout(std::time::Duration::from_secs(7), async {
            loop {
                let attempt = c.attempts.get(&request.id).await.unwrap();
                let recovery_finished = !scenario.starts_with("queued")
                    || attempt.execution == "rejected"
                    || attempt.notices.iter().any(|notice| {
                        notice.contains("bounded resource lookup")
                            || notice.contains("Synthetic recovery lookup failure")
                    });
                if attempt.preservation["contract"] != "pending" && recovery_finished {
                    break attempt;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let duplicate = c.submit(request.clone()).await.unwrap();
        let mut retry = request;
        retry.id = "another".into();
        let blocked = c.submit(retry).await.is_err();
        let calls = std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap_or_default()
            .lines()
            .count();
        finish_host(&c).await;
        c.assets.database.shutdown().await.unwrap();
        assert_ne!(saved.execution, "rejected", "{scenario}");
        assert!(saved.result.is_none());
        assert!(saved.observations.get("spent_usd").is_none());
        assert_eq!(duplicate.id, saved.id);
        assert!(blocked, "uncertainty or a dead session must not dispatch");
        assert_eq!(calls, usize::from(scenario == "exit-during-call"));
    }
}

#[tokio::test]
async fn dropping_submission_requester_does_not_abandon_the_owned_dispatch() {
    let (dir, c, request) = setup("normal").await;
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    let db = c.assets.database.clone();
    let blocker = tokio::spawn(async move {
        db.call(move |_| {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            Ok(())
        })
        .await
        .unwrap();
    });
    waiting.await.unwrap();
    let mut pending = Box::pin(c.submit(request.clone()));
    // Poll submission once while the database worker is held, then lose its requester.
    tokio::select! {
        biased;
        _ = &mut pending => panic!("submission cannot complete while its database is held"),
        _ = tokio::task::yield_now() => (),
    }
    drop(pending);
    release.send(()).unwrap();
    blocker.await.unwrap();
    let completed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Ok(saved) = c.attempts.get(&request.id).await
                && saved.preservation["contract"] == "complete"
            {
                break saved;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    finish_host(&c).await;
    c.assets.database.shutdown().await.unwrap();
    let saved = completed.expect("owned submission must finish after its requester disconnects");
    assert_eq!(saved.execution, "completed");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[tokio::test]
async fn retained_preparation_is_immutable_idempotent_and_transferred_once() {
    use wafflery::library::cleanup::{Cleanup, Selection};
    let (dir, c, request) = setup("normal").await;
    let receipt = c.prepare(request.clone()).await.unwrap();
    assert_eq!(
        receipt.expires_ms,
        c.prepare(request.clone()).await.unwrap().expires_ms
    );
    assert!(
        c.attempts
            .history(None, 50, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    let mut changed = request.clone();
    changed.params["label"] = json!("changed");
    assert!(c.prepare(changed.clone()).await.is_err());
    assert!(c.submit_prepared(changed).await.is_err());
    assert!(
        c.submit(request.clone()).await.is_err(),
        "direct dispatch must not replace a preparation"
    );
    let selection = Selection {
        unused_assets: true,
        ..Default::default()
    };
    let preview = Cleanup(c.assets.clone())
        .preview(selection.clone())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            c.cleanup(selection, preview.token)
        )
        .await
        .unwrap()
        .is_err()
    );
    std::fs::write(dir.path().join("original.png"), b"source changed").unwrap();
    let (first, duplicate) = tokio::join!(
        c.submit_prepared(request.clone()),
        c.submit_prepared(request.clone())
    );
    let saved = first.unwrap();
    assert_eq!(saved.id, duplicate.unwrap().id);
    assert_eq!(
        saved.arguments["source"]["path"],
        c.assets
            .root()
            .join("execution/attempt-1/input-0.png")
            .display()
            .to_string()
    );
    assert!(c.preparation(&request.id).await.is_err());
    c.discard_preparation(&request.id).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if c.attempts.get(&request.id).await.unwrap().preservation["contract"] == "complete" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    finish_host(&c).await;
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn discarded_or_restarted_preparations_cannot_fall_back_to_new_materialization() {
    let (dir, c, mut request) = setup("normal").await;
    c.prepare(request.clone()).await.unwrap();
    c.discard_preparation(&request.id).await.unwrap();
    assert!(c.submit_prepared(request.clone()).await.is_err());
    assert!(!c.assets.root().join("execution/attempt-1").exists());
    request.id = "before-restart".into();
    c.prepare(request.clone()).await.unwrap();
    let replacement = c.restart("example", true).await.unwrap();
    assert!(c.preparation(&request.id).await.is_err());
    assert!(!c.assets.root().join("execution/before-restart").exists());
    assert!(c.submit_prepared(request.clone()).await.is_err());
    request.id = "after-restart".into();
    request.revision = replacement.revision;
    c.prepare(request).await.unwrap();
    finish_host(&c).await;
    assert!(!c.assets.root().join("execution/after-restart").exists());
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    assert!(
        c.attempts
            .history(None, 50, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn paid_preparation_creates_no_attempt_and_cannot_bypass_the_paid_guard() {
    let (dir, c, request) = setup("paid").await;
    c.prepare(request.clone()).await.unwrap();
    assert!(c.submit_prepared(request).await.is_err());
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    assert!(
        c.attempts
            .history(None, 50, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    finish_host(&c).await;
    c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn preparation_stays_available_while_running_and_rejects_excess_before_copying() {
    let (dir, c, request) = setup("held").await;
    c.submit(request.clone()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while c.attempts.get(&request.id).await.unwrap().execution != "running" {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let drafts = (0..17).map(|index| {
        let c = c.clone();
        let mut draft = request.clone();
        draft.id = format!("draft-{index}");
        async move {
            let id = draft.id.clone();
            (id, c.prepare(draft).await)
        }
    });
    let results = futures_util::future::join_all(drafts).await;
    assert_eq!(
        results.iter().filter(|(_, result)| result.is_ok()).count(),
        16
    );
    let refused = &results
        .iter()
        .find(|(_, result)| result.is_err())
        .unwrap()
        .0;
    assert!(!c.assets.root().join("execution").join(refused).exists());
    assert_eq!(
        c.attempts
            .history(None, 50, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    std::fs::write(dir.path().join("runs/release-run"), b"finish").unwrap();
    finish_host(&c).await;
    for (id, _) in results {
        assert!(!c.assets.root().join("execution").join(id).exists());
    }
    c.assets.database.shutdown().await.unwrap();
}
