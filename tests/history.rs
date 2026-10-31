use serde_json::json;
use wafflehouse::library::{
    assets::AssetStore,
    attempts::{AttemptStore, Submission},
};

#[tokio::test]
async fn history_pages_are_stable_compact_and_survive_restart() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    for n in 0..5 {
        let request: Submission = serde_json::from_value(json!({"id":format!("a-{n}"),"tool":"sample","action":"mark","revision":"v1","params":{},"images":{}})).unwrap();
        store
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
    }
    store
        .update("a-4", |a| {
            a.journal = Some(json!({"large":"x".repeat(100_000)}));
            Ok(())
        })
        .await
        .unwrap();
    let first = store.history(None, 2, None, None).await.unwrap();
    assert_eq!(
        first.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        vec!["a-4", "a-3"]
    );
    assert!(serde_json::to_vec(&first).unwrap().len() < 2000);
    let cursor = first.last().unwrap().sequence;
    assets.database.shutdown().await.unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    store.recover().await.unwrap();
    let second = store
        .history(Some(cursor), 2, Some("sample".into()), Some("mark".into()))
        .await
        .unwrap();
    assert_eq!(
        second.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        vec!["a-2", "a-1"]
    );
    assert_eq!(second[0].execution, "interrupted");
    assert!(
        store
            .history(None, 20, Some("different".into()), None)
            .await
            .unwrap()
            .is_empty()
    );
    assets.database.shutdown().await.unwrap();
}
