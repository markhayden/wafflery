use serde_json::json;
use wafflery::library::{
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

#[tokio::test]
async fn schema_six_examples_are_backed_up_and_missing_native_occurrences_are_labelled_dependent() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let library = root.path().join("Library");
    std::fs::create_dir(&library).unwrap();
    std::fs::set_permissions(&library, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = library.join("library.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    for sql in [
        include_str!("../src/library/migrations/001-library.sql"),
        include_str!("../src/library/migrations/002-assets.sql"),
        include_str!("../src/library/migrations/003-attempts.sql"),
        include_str!("../src/library/migrations/004-baselines.sql"),
        include_str!("../src/library/migrations/005-examples.sql"),
        include_str!("../src/library/migrations/006-native-blobs.sql"),
    ] {
        db.execute_batch(sql).unwrap();
    }
    let record = json!({"id":"old","sequence":1,"name":"Legacy native example","notes":"Frozen provenance","request":{"id":"source","tool":"peer","action":"soften","revision":"old","params":{"amount":0.25},"images":{},"bindings":{"token":[{"kind":"artifact","reference":{"identity":"native"}}]},"lineage":{"token":[{"attempt":"deleted","port":"token","occurrence":0}]}},"inputs":{},"declaration":{}});
    db.execute(
        "INSERT INTO examples(id,record) VALUES('old',?1)",
        [serde_json::to_string(&record).unwrap()],
    )
    .unwrap();
    db.pragma_update(None, "user_version", 6).unwrap();
    drop(db);
    let assets = AssetStore::open(&library).unwrap();
    let saved = ExampleStore(assets.database.clone())
        .get("old".into())
        .await
        .unwrap();
    assert!(saved.workspace_dependent);
    assert!(saved.input_artifacts.is_empty());
    assert_eq!(saved.request.params["amount"], 0.25);
    assert!(!saved.request.lineage.is_empty());
    let backup = std::fs::read_dir(&library)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-v7-")
        })
        .unwrap();
    let old = rusqlite::Connection::open(backup).unwrap();
    assert_eq!(
        old.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assets.database.shutdown().await.unwrap();
}
