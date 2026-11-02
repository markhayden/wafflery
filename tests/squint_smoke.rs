//! Explicit local testkit executable only; no credentials or live provider calls.
#[path = "support/real_peers.rs"]
mod peer;
use serde_json::json;
use std::sync::Arc;
use wafflery::{
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};

#[tokio::test]
#[ignore = "requires WAFFLERY_SQUINT_BIN pointing to a contract 0.3.0 testkit binary"]
async fn real_squint_scripted_check_preserves_zero_spend_evidence() {
    let binary = peer::binary("WAFFLERY_SQUINT_BIN");
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir(&tool).unwrap();
    let config = tool.join("config.toml");
    let script = tool.join("held.json");
    std::fs::write(&config, include_str!("../testkit/squint/config.toml")).unwrap();
    std::fs::write(
        &script,
        include_str!("../testkit/squint/references-held.json"),
    )
    .unwrap();
    let input = dir.path().join("original.png");
    let bytes = include_bytes!("../ui/tests/fixtures/sample.png");
    std::fs::write(&input, bytes).unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let image = assets.import_path(input.clone()).await.unwrap();
    let reference_path = dir.path().join("reference.png");
    std::fs::write(
        &reference_path,
        include_bytes!("../ui/tests/fixtures/panorama.png"),
    )
    .unwrap();
    let reference = assets.import_path(reference_path).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            assets,
            vec![Descriptor {
                config: Some(config),
                id: "squint".into(),
                executable: binary,
                args: vec![
                    "serve".into(),
                    "--provider-script".into(),
                    script.display().to_string(),
                    "--out".into(),
                    tool.join("runs").display().to_string(),
                ],
                cwd: tool.clone(),
                roots: vec![tool.clone()],
            }],
        )
        .await
        .unwrap(),
    );
    let saved = peer::run(&c, async {
        let description = c.description("squint").await.unwrap();
        assert_eq!(description.document["contract"], "0.3.0");
        for action in description.document["actions"].as_array().unwrap() {
            assert_eq!(action["cost"]["may_spend"], false, "{action}");
            assert_eq!(action["readiness"]["state"], "ready", "{action}");
        }
        assert!(
            !description.document["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["name"] == "estimate")
        );
        assert!(description.schemas.contains_key("estimate"));
        let id = "squint-probe";
        let request = serde_json::from_value(json!({
            "id":id, "tool":"squint", "action":"reference_consistency",
            "revision":description.revision, "params":{},
            "images":{"candidate":[image.id],"references":[reference.id,image.id,reference.id]}
        }))
        .unwrap();
        c.submit(request).await.unwrap();
        let saved = peer::settled(&c, id).await;
        peer::reports(&c, &saved).await;
        saved
    })
    .await;
    assert_eq!(
        saved.execution,
        "completed",
        "{:?}; {:?}",
        saved.notices,
        saved.result.as_ref().map(|v| &v["error"])
    );
    assert_eq!(
        saved.preservation["contract"], "complete",
        "{:?}",
        saved.notices
    );
    assert!(saved.wire.is_some());
    let result = saved.result.unwrap();
    assert_eq!(result["spend"]["usd"], 0.0);
    assert_eq!(result["body"]["verdict"], "pass");
    let wire = &saved.wire.as_ref().unwrap()["params"]["arguments"]["references"];
    assert_eq!(wire.as_array().unwrap().len(), 3);
    assert_eq!(wire[0]["expect"]["file_sha256"], reference.sha256);
    assert_eq!(wire[1]["expect"]["file_sha256"], image.sha256);
    assert_eq!(wire[2]["expect"], wire[0]["expect"]);
    assert_ne!(wire[0]["path"], wire[2]["path"]);
    for (index, asset) in [reference.id.clone(), image.id.clone(), reference.id]
        .iter()
        .enumerate()
    {
        assert_eq!(saved.inputs["references"][index]["asset"]["id"], *asset);
    }
    assert!(saved.outputs.is_empty());
    assert_eq!(std::fs::read(input).unwrap(), bytes);
}
