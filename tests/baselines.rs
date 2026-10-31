use serde_json::json;
use wafflehouse::library::{
    assets::AssetStore,
    attempts::{AttemptStore, Submission},
    baselines::BaselineStore,
};
#[tokio::test]
async fn baseline_pins_require_a_completed_run_in_the_same_context_and_persist() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("Library");
    let assets = AssetStore::open(&root).unwrap();
    let attempts = AttemptStore::new(assets.database.clone());
    let request: Submission = serde_json::from_value(
        json!({"id":"one","tool":"sample","action":"mark","revision":"r","params":{},"images":{}}),
    )
    .unwrap();
    attempts
        .create(request, json!({}), json!({}), json!({}))
        .await
        .unwrap();
    let pins = BaselineStore(assets.database.clone());
    assert!(
        pins.set("sample".into(), "mark".into(), Some("one".into()))
            .await
            .is_err()
    );
    attempts
        .update("one", |a| {
            a.execution = "completed".into();
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        pins.set("other".into(), "mark".into(), Some("one".into()))
            .await
            .is_err()
    );
    pins.set("sample".into(), "mark".into(), Some("one".into()))
        .await
        .unwrap();
    assets.database.shutdown().await.unwrap();
    let assets = AssetStore::open(&root).unwrap();
    let pins = BaselineStore(assets.database.clone());
    assert_eq!(
        pins.get("sample".into(), "mark".into())
            .await
            .unwrap()
            .as_deref(),
        Some("one")
    );
    pins.set("sample".into(), "mark".into(), None)
        .await
        .unwrap();
    assert!(
        pins.get("sample".into(), "mark".into())
            .await
            .unwrap()
            .is_none()
    );
    assets.database.shutdown().await.unwrap();
}
