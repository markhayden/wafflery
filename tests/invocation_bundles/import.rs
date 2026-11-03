use super::*;
use wafflery::tools::{
    bundle_reader,
    bundles::{self, Options},
};
pub(super) async fn owned(h: &Host, id: &str) -> Attempt {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let saved = h.c.attempts.get(id).await.unwrap();
            if saved.preservation["native"] == "preserved" {
                return saved;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn explicit_bundle_import_uses_exact_native_files_once_per_identity_and_creates_no_runs() {
    let source = Host::native(&[]).await;
    let mut seed = source.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![source.image.id.clone()]);
    source.c.submit(seed).await.unwrap();
    let seed = owned(&source, "seed").await;
    let reference = &seed.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"];
    let binding = json!({"kind":"artifact","reference":reference});
    source
        .c
        .submit(
            source
                .request(
                    "collect",
                    "collect_tokens",
                    json!({"tokens":[binding,binding]}),
                )
                .await,
        )
        .await
        .unwrap();
    let saved = owned(&source, "collect").await;
    let bundle = bundles::build(&source.c.assets, &saved, &Options::default())
        .await
        .unwrap();
    let parsed = Arc::new(bundle_reader::read(std::fs::read(bundle.file.path()).unwrap()).unwrap());
    source.close().await;
    let target = Host::native(&[]).await;
    for repetition in 0..2 {
        let job = target
            .c
            .start_bundle_import(parsed.clone(), 0, "peer".into())
            .await
            .unwrap();
        let status = tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let state = job.snapshot();
                if state.phase != "importing" {
                    return state;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(status.phase, "completed", "{:?}", status.error);
        assert_eq!(status.inputs.len(), 2);
        assert!(
            status
                .inputs
                .iter()
                .all(|input| input.status == "completed")
        );
        let calls =
            std::fs::read_to_string(target.root.path().join("peer/control-calls.jsonl")).unwrap();
        assert_eq!(
            calls
                .lines()
                .filter(|line| serde_json::from_str::<Value>(line).unwrap()["name"] == "import")
                .count(),
            repetition + 1
        );
        assert!(
            target
                .c
                .attempts
                .history(None, 100, None, None)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            target.c.description("peer").await.unwrap().tool_session["spent_usd"],
            0
        );
    }
    target.close().await;
}

#[tokio::test]
async fn a_later_native_refusal_preserves_completed_inputs_and_the_original_archive() {
    use std::io::Read;
    let source = Host::native(&[]).await;
    let mut seed = source.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![source.image.id.clone()]);
    source.c.submit(seed).await.unwrap();
    let seed = owned(&source, "seed").await;
    let first = seed.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone();
    source
        .c
        .submit(
            source
                .request(
                    "soften",
                    "soften_token",
                    json!({"token":[{"kind":"artifact","reference":first}]}),
                )
                .await,
        )
        .await
        .unwrap();
    let second =
        owned(&source, "soften").await.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    assert_ne!(first["identity"], second["identity"]);
    source.c.submit(source.request("collect","collect_tokens",json!({"tokens":[{"kind":"artifact","reference":first},{"kind":"artifact","reference":second}]})).await).await.unwrap();
    let saved = owned(&source, "collect").await;
    let bundle = bundles::build(&source.c.assets, &saved, &Options::default())
        .await
        .unwrap();
    let bytes = std::fs::read(bundle.file.path()).unwrap();
    let mut files = vec![];
    for entry in tar::Archive::new(bytes.as_slice()).entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        let mut bytes = vec![];
        entry.read_to_end(&mut bytes).unwrap();
        files.push((name, bytes));
    }
    let mut manifest: Value = serde_json::from_slice(&files[0].1).unwrap();
    let name = manifest["steps"][0]["artifacts"][1]["member"]
        .as_str()
        .unwrap()
        .to_owned();
    let invalid = b"An opaque export that the tool itself must refuse".to_vec();
    use sha2::{Digest, Sha256};
    let hash: String = Sha256::digest(&invalid)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    manifest["steps"][0]["artifacts"][1]["sha256"] = json!(hash);
    files[0].1 = serde_json::to_vec(&manifest).unwrap();
    files.iter_mut().find(|(n, _)| n == &name).unwrap().1 = invalid;
    let mut archive = tar::Builder::new(vec![]);
    for (name, bytes) in files {
        let mut header = tar::Header::new_ustar();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        archive
            .append_data(&mut header, name, bytes.as_slice())
            .unwrap();
    }
    let parsed = Arc::new(bundle_reader::read(archive.into_inner().unwrap()).unwrap());
    source.close().await;
    let target = Host::native(&[]).await;
    let job = target
        .c
        .start_bundle_import(parsed.clone(), 0, "peer".into())
        .await
        .unwrap();
    let status = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let state = job.snapshot();
            if state.phase != "importing" {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(status.phase, "failed");
    assert_eq!(status.inputs[0].status, "completed");
    assert!(status.inputs[0].binding.is_some());
    assert_eq!(status.inputs[1].status, "failed");
    assert!(status.error.is_some());
    assert!(
        target
            .c
            .attempts
            .history(None, 10, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        wafflery::library::bundle_store::BundleStore(target.c.assets.clone())
            .get(parsed.sha256())
            .await
            .unwrap()
            .id,
        parsed.sha256()
    );
    target.close().await;
}

#[tokio::test]
async fn shutdown_during_one_import_retains_it_but_does_not_start_the_next_input() {
    let source = Host::native(&[]).await;
    let mut seed = source.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![source.image.id.clone()]);
    source.c.submit(seed).await.unwrap();
    let seed = owned(&source, "seed").await;
    let reference = &seed.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"];
    let location = fixtures::artifact("sample", "location")["record"].clone();
    source.c.submit(source.request("collect","collect_tokens",json!({"tokens":[{"kind":"artifact","reference":reference}],"guides":[{"kind":"location","value":location}]})).await).await.unwrap();
    let saved = owned(&source, "collect").await;
    let bundle = bundles::build(&source.c.assets, &saved, &Options::default())
        .await
        .unwrap();
    let parsed = Arc::new(bundle_reader::read(std::fs::read(bundle.file.path()).unwrap()).unwrap());
    source.close().await;
    let target = Host::native(&["--hold-control"]).await;
    let job = target
        .c
        .start_bundle_import(parsed, 0, "peer".into())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !target.root.path().join("peer/holding-control-0").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(target.c.restart("peer", true).await.is_err());
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    target.c.begin_shutdown(true, quit).await.unwrap();
    std::fs::write(target.root.path().join("peer/release-control"), b"ready").unwrap();
    let status = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let value = job.snapshot();
            if value.phase != "importing" {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(status.phase, "failed");
    assert_eq!(status.inputs[0].status, "completed");
    assert_eq!(status.inputs[1].status, "pending");
    assert!(status.error.unwrap().contains("shutting down"));
    tokio::time::timeout(Duration::from_secs(8), stopped.wait_for(|stopped| *stopped))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_dropped_http_response_keeps_import_progress_owned_and_blocks_review_dismissal() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use wafflery::host::{api, auth::Auth};
    let source = Host::native(&[]).await;
    let mut seed = source.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![source.image.id.clone()]);
    source.c.submit(seed).await.unwrap();
    let saved = owned(&source, "seed").await;
    let built = bundles::build(&source.c.assets, &saved, &Options::default())
        .await
        .unwrap();
    let bytes = std::fs::read(built.file.path()).unwrap();
    source.close().await;
    let target = Host::native(&[]).await;
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = api::router_with_tools(
        "http://127.0.0.1:45678",
        auth,
        quit,
        target.c.assets.clone(),
        Some(target.c.clone()),
    );
    let request = |method: &str, path: &str, bytes: Vec<u8>, mime: &str| {
        Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("authorization", format!("Bearer {}", session.token))
            .header("content-type", mime)
            .body(Body::from(bytes))
            .unwrap()
    };
    let uploaded = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/bundle-reviews",
            bytes,
            "application/x-tar",
        ))
        .await
        .unwrap();
    assert_eq!(uploaded.status(), StatusCode::CREATED);
    let uploaded: Value =
        serde_json::from_slice(&to_bytes(uploaded.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    let id = uploaded["id"].as_str().unwrap();
    // Hold publication after the explicit request starts, without a sleep or tool-specific flag.
    let publication = target.c.assets.storage.write().await;
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/bundle-reviews/{id}/import"),
            br#"{"step":0,"tool":"peer"}"#.to_vec(),
            "application/json",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    drop(response);
    let held = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/bundle-reviews/{id}/progress"),
            vec![],
            "application/json",
        ))
        .await
        .unwrap();
    let held: Value =
        serde_json::from_slice(&to_bytes(held.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    assert_eq!(held["phase"], "importing");
    let dismissed = app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/bundle-reviews/{id}"),
            vec![],
            "application/json",
        ))
        .await
        .unwrap();
    assert_eq!(dismissed.status(), StatusCode::CONFLICT);
    assert!(target.c.restart("peer", true).await.is_err());
    drop(publication);
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let response = app
                .clone()
                .oneshot(request(
                    "GET",
                    &format!("/api/bundle-reviews/{id}/progress"),
                    vec![],
                    "application/json",
                ))
                .await
                .unwrap();
            let status: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                    .unwrap();
            if status["phase"] != "importing" {
                assert_eq!(status["phase"], "completed", "{status}");
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        target
            .c
            .attempts
            .history(None, 10, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    target.close().await;
}
