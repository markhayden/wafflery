//! Resolved invocation is already part of the atomic result record, not a second mutable column.
#[path = "../testkit/v04_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
use serde_json::{Value, json};
use wafflery::{
    library::{
        assets::AssetStore,
        attempts::{AttemptStore, Submission},
    },
    tools::validate_contract,
};

#[tokio::test]
async fn every_terminal_invocation_survives_reopen_without_replacing_the_recorded_request() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("Library");
    let assets = AssetStore::open(&root).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    let mut expected = vec![];
    for (id, status, resolved) in [
        ("completed", "completed", true),
        ("cancelled", "cancelled", true),
        ("error", "error", true),
        ("invalid-resolved", "invalid", true),
        ("invalid-unresolved", "invalid", false),
    ] {
        let mut result = fixtures::result("test-prism", status);
        result["run"]["id"] = json!(id);
        result["run"]["resource"] = json!(format!("test-prism://run/{id}/run.json"));
        if resolved {
            result["invocation"] =
                fixtures::result("test-prism", "completed")["invocation"].clone();
            result["invocation"]["arguments"]["max_spend_usd"] = json!(0.1);
            result["invocation"]["effective"]["model"] = json!("fixture-model");
            result["invocation"]["requirements"] = json!(["TEST_MODEL_KEY", "config.toml"]);
        }
        validate_contract("urn:table-manners:core:result", &result).unwrap();
        let submitted =
            json!({"token":fixtures::artifact("test-prism", "test-prism:Token")["artifact"]});
        let request: Submission = serde_json::from_value(json!({
            "id":id,"tool":"test-prism","action":"soften_token","revision":"saved-revision",
            "params":{},"images":{}
        }))
        .unwrap();
        store
            .create(
                request.clone(),
                submitted.clone(),
                json!({"session":"saved-session","document":fixtures::description("test-prism")}),
                json!({}),
            )
            .await
            .unwrap();
        let wire = json!({"id":17,"method":"tools/call","params":{"name":"soften_token","arguments":submitted}});
        store.record_wire(id, wire.clone()).await.unwrap();
        store.bind_result(id, result.clone()).await.unwrap();
        let mut edited = request;
        edited.params = json!({"amount":0.75,"max_spend_usd":0.9});
        assert!(store.find(&edited).await.is_err());
        assert!(
            store
                .bind_result(id, fixtures::result("test-prism", "completed"))
                .await
                .is_err()
        );
        expected.push((id, result, wire));
    }
    assets.database.shutdown().await.unwrap();
    let assets = AssetStore::open(&root).unwrap();
    let store = AttemptStore::new(assets.database.clone());
    store.recover().await.unwrap();
    for (id, result, wire) in expected {
        let saved = store.get(id).await.unwrap();
        assert_eq!(saved.result, Some(result.clone()), "{id}");
        assert_eq!(saved.execution, result["status"].as_str().unwrap());
        assert_eq!(saved.wire, Some(wire));
        assert_eq!(saved.request.params, json!({}));
        assert!(saved.arguments.get("amount").is_none());
        assert!(saved.arguments.get("max_spend_usd").is_none());
        if id == "invalid-unresolved" {
            assert_eq!(result["invocation"], Value::Null);
        } else {
            assert_eq!(result["invocation"]["arguments"]["amount"], 0.25);
            assert_eq!(result["invocation"]["arguments"]["max_spend_usd"], 0.1);
            assert_eq!(
                result["invocation"]["requirements"],
                json!(["TEST_MODEL_KEY", "config.toml"])
            );
        }
    }
    assets.database.shutdown().await.unwrap();
}
