use super::*;
#[tokio::test]
async fn exports_and_idempotent_imports_are_controls_not_attempts() {
    let h = Host::new().await;
    h.c.submit(
        h.request(
            "portable",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    let saved = h.settled("portable").await;
    let exported = h.c.export_artifact("portable", "out", 0).await.unwrap();
    assert_eq!(
        exported.artifact,
        saved.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]
    );
    assert_eq!(exported.media_type, "application/json");
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    let first = h.c.import_artifact("portable", "out", 0).await.unwrap();
    assert_eq!(first["already_present"], false);
    let second = h.c.import_artifact("portable", "out", 0).await.unwrap();
    assert_eq!(second["already_present"], true);
    assert_eq!(second["artifact"], exported.artifact);
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    h.close().await;
}

async fn seed(h: &Host, id: &str) {
    h.c.submit(
        h.request(
            id,
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled(id).await;
}
async fn transfers(h: &Host, count: u64) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if h.c.snapshot().await.unwrap()["artifact_transfers"] == count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
async fn marker(h: &Host, index: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !h
            .root
            .path()
            .join(format!("peer/holding-control-{index}"))
            .exists()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn unsafe_export_headers_fail_before_native_publication() {
    use wafflery::library::blobs::BlobStore;
    for (fault, reason) in [
        ("hash", "fingerprint"),
        ("path", "outside registered evidence"),
        ("media", "application/json"),
    ] {
        let h = Host::with_flags(&["--export-fault", fault]).await;
        seed(&h, "source").await;
        let error = h.c.export_artifact("source", "out", 0).await.unwrap_err();
        assert!(error.to_string().contains(reason), "{error}");
        assert!(
            BlobStore(h.c.assets.clone())
                .list()
                .await
                .unwrap()
                .is_empty()
        );
        h.close().await;
    }
}
#[tokio::test]
async fn missing_exports_keep_the_structured_refusal_and_unportable_types_never_dispatch() {
    let h = Host::new().await;
    seed(&h, "source").await;
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    let error = h.c.export_artifact("source", "out", 0).await.err().unwrap();
    let wafflery::tools::Error::BindingRefusal(data) = error else {
        panic!("Expected binding refusal: {error}");
    };
    assert_eq!(data["bindings"][0]["port"], "artifact");
    assert!(data["bindings"][0]["remedy"].is_string());
    h.c.submit(h.request("memo", "summarize", json!({})).await)
        .await
        .unwrap();
    h.settled("memo").await;
    assert!(
        h.c.export_artifact("memo", "out", 0)
            .await
            .unwrap_err()
            .to_string()
            .contains("no declared portable")
    );
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        2
    );
    h.close().await;
}
#[tokio::test]
async fn controls_answer_during_a_run_without_taking_its_permit() {
    let h = Host::with_flags(&["--hold-operation"]).await;
    std::fs::write(h.root.path().join("peer/release-operation"), b"ready").unwrap();
    seed(&h, "source").await;
    std::fs::remove_file(h.root.path().join("peer/release-operation")).unwrap();
    h.c.submit(
        h.request(
            "held",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if h.c.snapshot().await.unwrap()["tools"][0]["progress"]["message"]
                .as_str()
                .is_some_and(|value| value.contains("Waiting"))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    h.c.export_artifact("source", "out", 0).await.unwrap();
    let response = h.c.import_artifact("source", "out", 0).await.unwrap();
    assert_eq!(response["already_present"], true);
    assert_eq!(h.c.snapshot().await.unwrap()["tools"][0]["busy"], true);
    h.c.cancel("held").await.unwrap();
    h.settled("held").await;
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        2
    );
    h.close().await;
}
#[tokio::test]
async fn dropped_waiters_keep_slots_owned_until_their_late_responses_are_drained() {
    use wafflery::library::{
        blobs::BlobStore,
        cleanup::{Cleanup, Selection},
    };
    let h = Host::with_flags(&["--hold-control"]).await;
    seed(&h, "source").await;
    let first = {
        let c = h.c.clone();
        tokio::spawn(async move { c.export_artifact("source", "out", 0).await })
    };
    marker(&h, 0).await;
    let second = {
        let c = h.c.clone();
        tokio::spawn(async move { c.export_artifact("source", "out", 0).await })
    };
    marker(&h, 1).await;
    transfers(&h, 2).await;
    assert!(
        h.c.export_artifact("source", "out", 0)
            .await
            .unwrap_err()
            .to_string()
            .contains("Two artifact transfers")
    );
    assert!(h.c.restart("peer", true).await.is_err());
    let selection = Selection::default();
    let preview = Cleanup(h.c.assets.clone())
        .preview(selection.clone())
        .await
        .unwrap();
    assert!(h.c.cleanup(selection, preview.token).await.is_err());
    first.abort();
    second.abort();
    let _ = first.await;
    let _ = second.await;
    transfers(&h, 2).await;
    std::fs::write(h.root.path().join("peer/release-control"), b"ready").unwrap();
    transfers(&h, 0).await;
    assert!(
        BlobStore(h.c.assets.clone())
            .list()
            .await
            .unwrap()
            .is_empty()
    );
    h.close().await;
}
#[tokio::test]
async fn a_cancelled_import_keeps_its_staged_file_until_reply_and_reconciles_by_identity() {
    let h = Host::with_flags(&["--hold-control", "--tiny-inline"]).await;
    seed(&h, "source").await;
    std::fs::write(h.root.path().join("peer/release-control"), b"ready").unwrap();
    h.c.export_artifact("source", "out", 0).await.unwrap();
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    std::fs::remove_file(h.root.path().join("peer/release-control")).unwrap();
    let import = {
        let c = h.c.clone();
        tokio::spawn(async move { c.import_artifact("source", "out", 0).await })
    };
    marker(&h, 0).await;
    let stage = std::fs::read_dir(h.c.assets.root().join("execution"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("artifact-import-")
        })
        .unwrap()
        .path();
    assert!(stage.join("artifact.native").is_file());
    import.abort();
    let _ = import.await;
    assert!(stage.join("artifact.native").is_file());
    std::fs::write(h.root.path().join("peer/release-control"), b"ready").unwrap();
    transfers(&h, 0).await;
    assert!(!stage.exists());
    assert_eq!(
        h.c.import_artifact("source", "out", 0).await.unwrap()["already_present"],
        true
    );
    h.close().await;
}

#[tokio::test]
async fn invalid_native_bytes_return_an_import_refusal_without_trying_a_preview() {
    use sha2::{Digest, Sha256};
    use wafflery::library::{
        blob_links::{BlobLink, Owner},
        blobs::BlobStore,
    };
    let h = Host::new().await;
    seed(&h, "source").await;
    let saved = h.c.attempts.get("source").await.unwrap();
    let bytes = include_bytes!("../../ui/tests/fixtures/sample.png").to_vec();
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let store = BlobStore(h.c.assets.clone());
    store.publish(bytes, hash.clone()).await.unwrap();
    store
        .attach(
            Owner::Attempt("source".into()),
            BlobLink {
                direction: "out".into(),
                occurrence: 0,
                sha256: hash,
                media_type: "application/json".into(),
                artifact: saved.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone(),
            },
        )
        .await
        .unwrap();
    let error = h.c.import_artifact("source", "out", 0).await.unwrap_err();
    let wafflery::tools::Error::ArtifactControlRefusal(data) = error else {
        panic!("Expected structured import refusal: {error}");
    };
    assert_eq!(data["reason"], "invalid_arguments");
    assert!(data["run"].is_null());
    assert_eq!(data["error"]["retryable"], false);
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    h.close().await;
}
#[tokio::test]
async fn artifact_api_uses_saved_occurrences_and_rejects_arbitrary_paths() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use wafflery::host::{api, auth::Auth};
    let h = Host::new().await;
    seed(&h, "source").await;
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let origin = "http://127.0.0.1:45678";
    let app = api::router_with_tools(origin, auth, quit, h.c.assets.clone(), Some(h.c.clone()));
    for (body, status) in [
        (r#"{"path":"/etc/hosts"}"#, StatusCode::UNPROCESSABLE_ENTITY),
        ("{}", StatusCode::OK),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/api/attempts/source/artifacts/out/0/export")
            .header("host", "127.0.0.1:45678")
            .header("origin", origin)
            .header("authorization", format!("Bearer {}", session.token))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), status);
        if status == StatusCode::OK {
            let value: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            assert_eq!(value["direction"], "out");
            assert!(value.get("path").is_none());
        }
    }
    h.close().await;
}

#[tokio::test]
async fn shutdown_waits_for_an_owned_control_instead_of_closing_its_tool() {
    let h = Host::with_flags(&["--hold-control"]).await;
    seed(&h, "source").await;
    let control = {
        let c = h.c.clone();
        tokio::spawn(async move { c.export_artifact("source", "out", 0).await })
    };
    marker(&h, 0).await;
    control.abort();
    let _ = control.await;
    let (quit, mut finished) = tokio::sync::watch::channel(false);
    h.c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if h.c.shutdown_status().await.phase == "waiting" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!*finished.borrow());
    transfers(&h, 1).await;
    std::fs::write(h.root.path().join("peer/release-control"), b"ready").unwrap();
    tokio::time::timeout(Duration::from_secs(8), finished.wait_for(|done| *done))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(h.c.snapshot().await.unwrap()["artifact_transfers"], 0);
}
