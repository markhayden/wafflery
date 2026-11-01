use serde_json::json;
use wafflery::library::{
    assets::AssetStore,
    attempts::{AttemptStore, Submission},
    baselines::BaselineStore,
    cleanup::{Cleanup, Selection},
    examples::ExampleStore,
};
#[tokio::test]
async fn cleanup_rechecks_references_preserves_examples_and_clears_baselines() {
    let dir = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let original = dir.path().join("original.png");
    std::fs::write(&original, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(original.clone()).await.unwrap();
    let attempts = AttemptStore::new(assets.database.clone());
    let request:Submission=serde_json::from_value(json!({"id":"source","tool":"sample","action":"mark","revision":"r","params":{},"images":{"source":[asset.id]}})).unwrap();
    attempts
        .create(request, json!({}), json!({}), json!({}))
        .await
        .unwrap();
    attempts
        .update("source", |a| {
            a.execution = "completed".into();
            Ok(())
        })
        .await
        .unwrap();
    let pins = BaselineStore(assets.database.clone());
    pins.set("sample".into(), "mark".into(), Some("source".into()))
        .await
        .unwrap();
    let cleanup = Cleanup(assets.clone());
    let selection = Selection {
        attempts: vec!["source".into()],
        ..Default::default()
    };
    let stale = cleanup.preview(selection.clone()).await.unwrap();
    assert_eq!(stale.reclaimable_bytes, asset.size as u64);
    let examples = ExampleStore(assets.database.clone());
    let example = examples
        .save("source".into(), "Keep".into(), "".into())
        .await
        .unwrap();
    assert!(
        cleanup
            .confirm(selection.clone(), stale.token)
            .await
            .is_err()
    );
    assert!(attempts.get("source").await.is_ok());
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    assert_eq!(preview.reclaimable_bytes, 0);
    assert_eq!(preview.baselines_cleared, 1);
    cleanup.confirm(selection, preview.token).await.unwrap();
    assert!(attempts.get("source").await.is_err());
    assert!(
        pins.get("sample".into(), "mark".into())
            .await
            .unwrap()
            .is_none()
    );
    assert!(examples.get(example.id.clone()).await.is_ok());
    assert!(assets.read(&asset.id).await.is_ok());
    let selection = Selection {
        examples: vec![example.id],
        ..Default::default()
    };
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    let lease = assets.storage.read().await;
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(25),
            cleanup.confirm(selection.clone(), preview.token.clone())
        )
        .await
        .is_err()
    );
    drop(lease);
    cleanup.confirm(selection, preview.token).await.unwrap();
    assert!(assets.read(&asset.id).await.is_err());
    assert_eq!(
        std::fs::read(&original).unwrap(),
        include_bytes!("../ui/tests/fixtures/sample.png")
    );
    let imported = assets.import_path(original.clone()).await.unwrap();
    let path = assets
        .root()
        .join("assets/sha256")
        .join(&imported.sha256[..2])
        .join(&imported.sha256);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let selection = Selection {
        unused_assets: true,
        ..Default::default()
    };
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    let failed = cleanup.confirm(selection, preview.token).await.unwrap();
    assert_eq!(failed.failures.len(), 1);
    assert_eq!(assets.list().await.unwrap().len(), 1);
    assets.database.shutdown().await.unwrap();
}
