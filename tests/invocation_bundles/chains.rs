use super::bundle_import_cases::owned;
use super::*;
use wafflery::tools::{bundle_reader, bundles, chain_edges};
async fn seed(h: &Host, id: &str) -> Attempt {
    let mut request = h.request(id, "make_token", json!({})).await;
    request
        .images
        .insert("source".into(), vec![h.image.id.clone()]);
    h.c.submit(request).await.unwrap();
    let saved = owned(h, id).await;
    h.settled(id).await;
    saved
}
async fn soften(h: &Host, id: &str, source: &Attempt) -> Attempt {
    let reference = source.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let mut request = h
        .request(
            id,
            "soften_token",
            json!({"token":[{"kind":"artifact","reference":reference}]}),
        )
        .await;
    request.lineage = serde_json::from_value(
        json!({"token":[{"attempt":source.id,"port":"token","occurrence":0}]}),
    )
    .unwrap();
    h.c.submit(request).await.unwrap();
    let saved = owned(h, id).await;
    h.settled(id).await;
    saved
}
fn controls(h: &Host) -> String {
    std::fs::read_to_string(h.root.path().join("peer/control-calls.jsonl")).unwrap_or_default()
}
#[tokio::test]
async fn selected_chains_keep_order_input_copies_and_deterministic_bytes_without_running_provenance()
 {
    let h = Host::native(&[]).await;
    let first = seed(&h, "seed").await;
    let second = soften(&h, "soften-a", &first).await;
    h.settled("soften-a").await;
    let third = soften(&h, "soften-b", &first).await;
    h.settled("soften-b").await;
    let before = controls(&h);
    let selection = vec![first.clone(), second.clone(), third.clone()];
    let a = bundles::build_chain(&h.c.assets, &selection).await.unwrap();
    let b = bundles::build_chain(&h.c.assets, &selection).await.unwrap();
    assert_eq!(a.sha256, b.sha256);
    let parsed = bundle_reader::read(std::fs::read(a.file.path()).unwrap()).unwrap();
    assert_eq!(parsed.manifest()["steps"].as_array().unwrap().len(), 3);
    assert_eq!(
        parsed.manifest()["steps"][0]["invocation"]["action"],
        "make_token"
    );
    assert_eq!(
        parsed.manifest()["steps"][1]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        parsed.manifest()["steps"][2]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let edges = chain_edges::resolve(parsed.manifest()).unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|edge| edge.producer.step == 0));
    assert!(!a.workspace_dependent);
    assert_eq!(controls(&h), before);
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        3
    );
    let single = bundles::build_chain(&h.c.assets, &[second]).await.unwrap();
    assert!(chain_edges::resolve(&single.manifest).unwrap().is_empty());
    assert_eq!(
        single.manifest["steps"][0]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let store = wafflery::library::bundle_store::BundleStore(h.c.assets.clone());
    let ids = selection
        .iter()
        .map(|attempt| attempt.id.clone())
        .collect::<Vec<_>>();
    let record = store.save_chain(&ids, a).await.unwrap();
    for id in &ids {
        assert_eq!(
            store
                .list(None, Some(id.clone()))
                .await
                .unwrap()
                .items
                .len(),
            1
        );
    }
    let cleanup = wafflery::library::cleanup::Selection {
        attempts: ids,
        unused_assets: true,
        ..Default::default()
    };
    let preview = wafflery::library::cleanup::Cleanup(h.c.assets.clone())
        .preview(cleanup.clone())
        .await
        .unwrap();
    assert!(!preview.assets.iter().any(|asset| asset.id == h.image.id));
    h.c.cleanup(cleanup, preview.token).await.unwrap();
    h.close().await;
    std::fs::remove_dir_all(h.root.path().join("peer")).unwrap();
    let (mut file, size) = store.open(&record.id).await.unwrap();
    use std::io::Read;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(size, bytes.len() as u64);
    assert_eq!(
        bundle_reader::read(bytes).unwrap().manifest()["steps"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
#[tokio::test]
async fn coincidental_selected_producers_reversed_order_and_duplicate_runs_are_refused_without_calls()
 {
    let h = Host::native(&[]).await;
    let actual = seed(&h, "actual").await;
    h.settled("actual").await;
    let unrelated = seed(&h, "unrelated").await;
    h.settled("unrelated").await;
    let consumer = soften(&h, "consumer", &actual).await;
    h.settled("consumer").await;
    assert_eq!(
        actual.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]["identity"],
        unrelated.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]["identity"]
    );
    let before = controls(&h);
    for selection in [
        vec![unrelated, consumer.clone()],
        vec![consumer, actual.clone()],
        vec![actual.clone(), actual.clone()],
    ] {
        assert!(bundles::build_chain(&h.c.assets, &selection).await.is_err());
    }
    assert_eq!(controls(&h), before);
    h.close().await;
}

#[tokio::test]
async fn chain_api_requires_bounded_distinct_ids_and_returns_derived_edges_without_dispatch() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use wafflery::host::{api, auth::Auth};
    let h = Host::native(&[]).await;
    let a = seed(&h, "api-seed").await;
    let b = soften(&h, "api-consumer", &a).await;
    let before = controls(&h);
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
        h.c.assets.clone(),
        Some(h.c.clone()),
    );
    let request = |body: Value| {
        Request::builder()
            .method("POST")
            .uri("/api/bundles")
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    for wrong in [
        json!({"attempts":[]}),
        json!({"attempts":[a.id,a.id]}),
        json!({"attempts":(0..17).map(|n|format!("run-{n}")).collect::<Vec<_>>()}),
        json!({"attempts":[a.id],"path":"/other"}),
    ] {
        assert_eq!(
            app.clone().oneshot(request(wrong)).await.unwrap().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let response = app
        .clone()
        .oneshot(request(json!({"attempts":[a.id,b.id]})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let record: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert!(record["chain"]["problem"].is_null());
    assert_eq!(record["chain"]["edges"][0]["producer"]["step"], 0);
    assert_eq!(record["chain"]["edges"][0]["consumer"], 1);
    assert_eq!(record["manifest"]["steps"].as_array().unwrap().len(), 2);
    assert_eq!(controls(&h), before);
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
