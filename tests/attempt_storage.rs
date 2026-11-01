use serde_json::json;
use wafflehouse::library::{
    assets::AssetStore,
    attempts::{AttemptStore, Submission},
};
#[tokio::test]
async fn submission_is_immutable_idempotent_and_restart_never_replays_it() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    let submission: Submission = serde_json::from_value(json!({"id":"attempt-1","tool":"fixture","action":"mark_band","revision":"a","params":{"label":"original"},"images":{}})).unwrap();
    store
        .create(
            submission.clone(),
            json!({"label":"original"}),
            json!({}),
            json!({}),
        )
        .await
        .unwrap();
    let saved = store.find(&submission).await.unwrap().unwrap();
    assert_eq!(saved.request.params["label"], "original");
    let mut changed = submission.clone();
    changed.params = json!({"label":"different"});
    assert!(store.find(&changed).await.is_err());
    store
        .record_wire(
            "attempt-1",
            json!({"id":7,"params":{"_meta":{"progressToken":4}}}),
        )
        .await
        .unwrap();
    assert_eq!(store.get("attempt-1").await.unwrap().wire.unwrap()["id"], 7);
    assets.database.shutdown().await.unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    store.recover().await.unwrap();
    let saved = store.get("attempt-1").await.unwrap();
    assert_eq!(saved.execution, "interrupted");
    assert_eq!(saved.request.params["label"], "original");
    assert_eq!(saved.wire.unwrap()["params"]["_meta"]["progressToken"], 4);
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn a_tool_run_resource_can_only_bind_once_within_the_same_process_session() {
    let dir = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    for id in ["first", "second"] {
        let submission: Submission = serde_json::from_value(
            json!({"id":id,"tool":"fixture","action":"op","revision":"a","params":{},"images":{}}),
        )
        .unwrap();
        store
            .create(
                submission,
                json!({}),
                json!({"session":"same-process"}),
                json!({}),
            )
            .await
            .unwrap();
    }
    let result = json!({"status":"completed","run":{"resource":"fixture://run/one/run.json"}});
    store.bind_result("first", result.clone()).await.unwrap();
    assert!(store.bind_result("second", result).await.is_err());
    assert!(store.get("second").await.unwrap().result.is_none());
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn cancelling_a_prepared_attempt_atomically_prevents_its_transport_record() {
    let dir = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    let submission: Submission = serde_json::from_value(
        json!({"id":"early","tool":"fixture","action":"op","revision":"a","params":{},"images":{}}),
    )
    .unwrap();
    store
        .create(submission, json!({}), json!({}), json!({}))
        .await
        .unwrap();
    let cancelled = store.request_cancel("early").await.unwrap();
    assert_eq!(cancelled.execution, "rejected");
    assert!(cancelled.result.is_none());
    assert!(store.record_wire("early", json!({"id":4})).await.is_err());
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn pre_v02_history_without_session_observations_remains_readable() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    let submission = serde_json::from_value(
        json!({"id":"old","tool":"fixture","action":"op","revision":"old","params":{},"images":{}}),
    )
    .unwrap();
    let saved = store
        .create(
            submission,
            json!({}),
            json!({"document":{"contract":"0.1.0"}}),
            json!({}),
        )
        .await
        .unwrap();
    let mut old = serde_json::to_value(saved).unwrap();
    old.as_object_mut().unwrap().remove("observations");
    old["execution"] = json!("completed");
    old["preservation"]["contract"] = json!("complete");
    old["result"] = json!({"contract":"0.1.0","status":"completed","run":{"id":"old","resource":"fixture://run/old/run.json","dir":null}});
    let expected = old["result"].clone();
    assets
        .database
        .call(move |db| {
            db.execute(
                "UPDATE attempts SET record=?1 WHERE id='old'",
                [old.to_string()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assets.database.shutdown().await.unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    store.recover().await.unwrap();
    let saved = store.get("old").await.unwrap();
    assert_eq!(saved.result.unwrap(), expected);
    assert_eq!(saved.execution, "completed");
    assert!(saved.observations.is_null());
    assets.database.shutdown().await.unwrap();
}
