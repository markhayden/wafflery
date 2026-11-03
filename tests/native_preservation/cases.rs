use super::*;
async fn native_done(h: &Host, id: &str) -> Attempt {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let saved = h.c.attempts.get(id).await.unwrap();
            if saved.preservation["native"] != "preserving"
                && saved.preservation["native"] != "unsupported"
            {
                return saved;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn native_inputs_and_outputs_are_owned_without_replaying_the_upstream_action() {
    use wafflery::library::{blob_links::Owner, blobs::BlobStore};
    let h = Host::native(&[]).await;
    h.c.submit(
        h.request(
            "location",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("location").await;
    let location = native_done(&h, "location").await;
    assert_eq!(location.preservation["native"], "preserved");
    let reference = location.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone();
    h.c.submit(h.request("token","stamp_marker",json!({"image":[{"kind":"image","asset":h.image.id}],"marker":[{"kind":"artifact","reference":reference}]})).await).await.unwrap();
    h.settled("token").await;
    let token = native_done(&h, "token").await;
    assert_eq!(token.preservation["native"], "preserved");
    assert_eq!(token.execution, "completed");
    let blobs = BlobStore(h.c.assets.clone());
    assert_eq!(blobs.list().await.unwrap().len(), 2);
    assert_eq!(
        blobs
            .links(Owner::Attempt("token".into()))
            .await
            .unwrap()
            .len(),
        2
    );
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    assert!(h.c.inspect_artifact("token", "out", 0).await.is_err());
    h.c.import_artifact("token", "out", 0).await.unwrap();
    h.c.inspect_artifact("token", "out", 0).await.unwrap();
    h.c.import_artifact("location", "out", 0).await.unwrap();
    h.c.inspect_artifact("location", "out", 0).await.unwrap();
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
fn export_calls(h: &Host) -> usize {
    std::fs::read_to_string(h.root.path().join("peer/control-calls.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter(|line| serde_json::from_str::<Value>(line).unwrap()["name"] == "export")
        .count()
}
#[tokio::test]
async fn duplicate_input_and_output_occurrences_share_one_native_export() {
    use wafflery::library::{blob_links::Owner, blobs::BlobStore};
    let h = Host::native(&[]).await;
    // The task's shared image slot is separate from operation ports.
    let mut seed = h.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![h.image.id.clone()]);
    h.c.submit(seed).await.unwrap();
    h.settled("seed").await;
    let seed = native_done(&h, "seed").await;
    let reference = seed.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let item = json!({"kind":"artifact","reference":reference});
    h.c.submit(
        h.request(
            "duplicates",
            "collect_tokens",
            json!({"tokens":[item,item]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("duplicates").await;
    let saved = native_done(&h, "duplicates").await;
    assert_eq!(saved.preservation["native"], "preserved");
    assert_eq!(export_calls(&h), 1);
    let blobs = BlobStore(h.c.assets.clone());
    assert_eq!(blobs.list().await.unwrap().len(), 1);
    assert_eq!(
        blobs
            .links(Owner::Attempt("duplicates".into()))
            .await
            .unwrap()
            .len(),
        4
    );
    h.close().await;
}
#[tokio::test]
async fn unportable_types_are_workspace_dependent_and_export_failures_never_change_run_outcomes() {
    let h = Host::native(&[]).await;
    h.c.submit(h.request("memo", "summarize", json!({})).await)
        .await
        .unwrap();
    h.settled("memo").await;
    let memo = native_done(&h, "memo").await;
    assert_eq!(memo.preservation["native"], "workspace-dependent");
    assert_eq!(export_calls(&h), 0);
    h.close().await;
    let h = Host::native(&["--export-fault", "hash"]).await;
    h.c.submit(
        h.request(
            "bad-export",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("bad-export").await;
    let saved = native_done(&h, "bad-export").await;
    assert_eq!(saved.execution, "completed");
    assert_eq!(saved.preservation["contract"], "complete");
    assert_eq!(saved.preservation["native"], "failed");
    assert!(
        saved.observations["native_artifacts"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("fingerprint")
    );
    h.close().await;
}

#[tokio::test]
async fn held_exports_do_not_hold_run_completion_and_explicit_retry_never_replays_actions() {
    let h = Host::native(&["--hold-control"]).await;
    for id in ["first", "duplicate"] {
        let mut request = h.request(id, "make_token", json!({})).await;
        request
            .images
            .insert("source".into(), vec![h.image.id.clone()]);
        h.c.submit(request).await.unwrap();
        let saved = h.settled(id).await;
        assert_eq!(saved.execution, "completed");
        assert_eq!(saved.preservation["native"], "preserving");
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while export_calls(&h) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        export_calls(&h),
        1,
        "the second worker coalesces the same reference"
    );
    h.c.queue_native_preservation("first").await.unwrap();
    let selection = wafflery::library::cleanup::Selection::default();
    let preview = wafflery::library::cleanup::Cleanup(h.c.assets.clone())
        .preview(selection.clone())
        .await
        .unwrap();
    assert!(h.c.cleanup(selection, preview.token).await.is_err());
    std::fs::write(h.root.path().join("peer/release-control"), b"").unwrap();
    for id in ["first", "duplicate"] {
        assert_eq!(
            native_done(&h, id).await.preservation["native"],
            "preserved"
        );
        h.c.queue_native_preservation(id).await.unwrap();
        assert_eq!(
            native_done(&h, id).await.preservation["native"],
            "preserved"
        );
    }
    assert_eq!(export_calls(&h), 1);
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
async fn retry_recovers_interrupted_status_from_owned_bytes_after_a_new_host_session() {
    let h = Host::native(&[]).await;
    h.c.submit(
        h.request(
            "saved",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("saved").await;
    native_done(&h, "saved").await;
    h.close().await;
    h.c.attempts
        .update("saved", |a| {
            a.preservation["native"] = json!("preserving");
            a.observations["native_artifacts"][0]["state"] = json!("preserving");
            Ok(())
        })
        .await
        .unwrap();
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab");
    let c = Arc::new(
        Coordinator::open(
            h.c.assets.clone(),
            vec![Descriptor {
                id: "peer".into(),
                executable: binary,
                args: vec![
                    h.root.path().join("peer").display().to_string(),
                    "v04-operations".into(),
                ],
                cwd: h.root.path().into(),
                allow_dirs: vec![h.root.path().into()],
                evidence_dirs: vec![h.root.path().into()],
                config: None,
            }],
        )
        .await
        .unwrap(),
    );
    assert_eq!(
        c.attempts.get("saved").await.unwrap().preservation["native"],
        "failed"
    );
    c.queue_native_preservation("saved").await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while c.attempts.get("saved").await.unwrap().preservation["native"] == "preserving" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        c.attempts.get("saved").await.unwrap().preservation["native"],
        "preserved"
    );
    assert!(c.inspect_artifact("saved", "out", 0).await.is_err());
    c.import_artifact_into("saved", "out", 0, Some("peer"))
        .await
        .unwrap();
    c.inspect_artifact_at("saved", "out", 0, Some("peer"))
        .await
        .unwrap();
    assert_eq!(
        export_calls(&h),
        1,
        "cached owned bytes need no upstream export"
    );
    assert_eq!(
        c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, tx).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), rx.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn restoration_uses_the_selected_registration_instead_of_the_source_store() {
    let h = Host::native(&[]).await;
    h.c.submit(
        h.request(
            "source",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("source").await;
    native_done(&h, "source").await;
    h.close().await;
    let descriptors = ["peer", "other"].map(|id| Descriptor {
        id: id.into(),
        executable: Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab"),
        args: vec![
            h.root.path().join(id).display().to_string(),
            "v04-operations".into(),
        ],
        cwd: h.root.path().into(),
        allow_dirs: vec![h.root.path().into()],
        evidence_dirs: vec![h.root.path().into()],
        config: None,
    });
    let c = Arc::new(
        Coordinator::open(h.c.assets.clone(), descriptors.into())
            .await
            .unwrap(),
    );
    c.inspect_artifact("source", "out", 0).await.unwrap();
    assert!(
        c.inspect_artifact_at("source", "out", 0, Some("other"))
            .await
            .is_err()
    );
    assert!(
        c.import_artifact_into("source", "out", 0, Some("not-installed"))
            .await
            .is_err()
    );
    c.import_artifact_into("source", "out", 0, Some("other"))
        .await
        .unwrap();
    c.inspect_artifact_at("source", "out", 0, Some("other"))
        .await
        .unwrap();
    assert!(h.root.path().join("other/artifacts").exists());
    assert_eq!(
        c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, tx).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), rx.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn retry_keeps_preserved_items_available_and_a_peer_exiting_mid_export_only_fails_preservation()
 {
    let h = Host::native(&[]).await;
    h.c.submit(
        h.request(
            "saved",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("saved").await;
    let saved = native_done(&h, "saved").await;
    assert_eq!(saved.preservation["native"], "preserved");
    let retried = h.c.queue_native_preservation("saved").await.unwrap();
    assert_eq!(retried.preservation["native"], "preserved");
    assert_eq!(
        retried.observations["native_artifacts"],
        saved.observations["native_artifacts"]
    );
    assert_eq!(h.c.snapshot().await.unwrap()["native_preservations"], 0);
    h.close().await;
    let h = Host::native(&["--export-fault", "exit"]).await;
    h.c.submit(
        h.request(
            "exited",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("exited").await;
    let saved = native_done(&h, "exited").await;
    assert_eq!(saved.execution, "completed");
    assert_eq!(saved.preservation["native"], "failed");
    assert_eq!(saved.result.as_ref().unwrap()["spend"]["usd"], 0);
    assert!(saved.observations["native_artifacts"][0]["reason"].is_string());
    h.close().await;
}
