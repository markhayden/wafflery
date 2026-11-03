use super::bundle_import_cases::owned;
use super::*;
use wafflery::{
    library::bundle_store::BundleStore,
    tools::{
        bundle_reader,
        bundles::{self, Options},
    },
};
#[tokio::test]
async fn an_imported_step_uses_fresh_preparation_and_records_provenance_without_mutating_the_archive()
 {
    let source = Host::native(&[]).await;
    let mut seed = source.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![source.image.id.clone()]);
    source.c.submit(seed).await.unwrap();
    let token =
        owned(&source, "seed").await.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    source
        .c
        .submit(
            source
                .request(
                    "soften",
                    "soften_token",
                    json!({"token":[{"kind":"artifact","reference":token}]}),
                )
                .await,
        )
        .await
        .unwrap();
    let original = owned(&source, "soften").await;
    let built = bundles::build(&source.c.assets, &original, &Options::default())
        .await
        .unwrap();
    let parsed = Arc::new(bundle_reader::read(std::fs::read(built.file.path()).unwrap()).unwrap());
    let manifest = parsed.manifest().clone();
    source.close().await;
    std::fs::remove_dir_all(source.root.path().join("peer/artifacts")).unwrap();
    let target = Host::native(&[]).await;
    let imported = target
        .c
        .start_bundle_import(parsed.clone(), 0, "peer".into())
        .await
        .unwrap();
    let state = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = imported.snapshot();
            if status.phase != "importing" {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(state.phase, "completed");
    let mut request = target
        .request(
            "replay",
            "soften_token",
            json!({"token":[state.inputs[0].binding]}),
        )
        .await;
    request.params = json!({"amount":0.75});
    request = serde_json::from_value({
        let mut value = serde_json::to_value(request).unwrap();
        value["source_bundle"] = json!({"id":parsed.sha256(),"step":0});
        value
    })
    .unwrap();
    let prepared = target.c.prepare(request.clone()).await.unwrap();
    assert_eq!(prepared.id, "replay");
    assert!(
        target
            .c
            .attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    target.c.submit_prepared(request).await.unwrap();
    let saved = owned(&target, "replay").await;
    target.settled("replay").await;
    assert_eq!(
        serde_json::to_value(&saved.request).unwrap()["source_bundle"],
        json!({"id":parsed.sha256(),"step":0})
    );
    assert_eq!(
        saved.result.as_ref().unwrap()["invocation"]["arguments"]["amount"],
        0.75
    );
    assert_ne!(
        saved.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]["identity"],
        original.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]["identity"]
    );
    assert_eq!(
        BundleStore(target.c.assets.clone())
            .get(parsed.sha256())
            .await
            .unwrap()
            .manifest,
        manifest
    );
    assert_eq!(
        BundleStore(target.c.assets.clone())
            .list(None, Some("replay".into()))
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    for (id, step, action) in [
        ("f".repeat(64), 0, "soften_token"),
        (parsed.sha256().into(), 9, "soften_token"),
        (parsed.sha256().into(), 0, "summarize"),
    ] {
        let mut value =
            serde_json::to_value(target.request("rejected", action, json!({})).await).unwrap();
        value["source_bundle"] = json!({"id":id,"step":step});
        let request: Submission = serde_json::from_value(value).unwrap();
        assert!(
            target
                .c
                .prepare(request.clone())
                .await
                .err()
                .expect("A bad bundle source must be refused")
                .to_string()
                .to_lowercase()
                .contains("bundle")
        );
        assert!(
            target
                .c
                .submit(request)
                .await
                .expect_err("A bad bundle source must be refused")
                .to_string()
                .to_lowercase()
                .contains("bundle")
        );
    }
    assert_eq!(
        target
            .c
            .attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    target.close().await;
}
