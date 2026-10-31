use serde_json::json;
use wafflehouse::library::{
    assets::AssetStore,
    attempts::{AttemptStore, Submission},
    examples::ExampleStore,
};
#[tokio::test]
async fn examples_snapshot_inputs_and_own_independent_references_without_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let image = dir.path().join("sample.png");
    std::fs::write(&image, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(image).await.unwrap();
    let attempts = AttemptStore::new(assets.database.clone());
    let request:Submission=serde_json::from_value(json!({"id":"source","tool":"sample","action":"mark","revision":"r","params":{"label":"saved"},"images":{"source":[asset.id]}})).unwrap();
    attempts
        .create(
            request,
            json!({}),
            json!({}),
            json!({"source":[{"asset":asset}]}),
        )
        .await
        .unwrap();
    let examples = ExampleStore(assets.database.clone());
    let saved = examples
        .save("source".into(), "Example one".into(), "Notes".into())
        .await
        .unwrap();
    attempts
        .update("source", |a| {
            a.request.params["label"] = json!("changed");
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            examples.get(saved.id.clone()).await.unwrap().request.params["label"],
            "saved"
        );
    }
    let count: i64 = assets
        .database
        .call(|db| Ok(db.query_row("SELECT count(*) FROM attempts", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(count, 1);
    let refs: i64 = assets
        .database
        .call(|db| Ok(db.query_row("SELECT count(*) FROM example_assets", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(refs, 1);
    assert!(
        examples
            .save("source".into(), " ".into(), "".into())
            .await
            .is_err()
    );
    assets.database.shutdown().await.unwrap();
}
