//! Explicit local testkit executable only; no credentials or live provider calls.
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use wafflery::{
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};

#[tokio::test]
#[ignore = "requires WAFFLERY_SQUINT_BIN pointing to a contract 0.3.0 testkit binary"]
async fn real_squint_scripted_check_preserves_zero_spend_evidence() {
    probe("source_fidelity", json!({})).await;
}

#[tokio::test]
#[ignore = "requires WAFFLERY_SQUINT_BIN pointing to a contract 0.3.0 testkit binary"]
async fn real_squint_estimate_preserves_complete_evidence() {
    probe("estimate", json!({"use_case":"source-fidelity"})).await;
}

async fn probe(action: &str, params: serde_json::Value) {
    let Some(binary) = std::env::var_os("WAFFLERY_SQUINT_BIN").map(PathBuf::from) else {
        eprintln!("SKIPPED: WAFFLERY_SQUINT_BIN not set");
        return;
    };
    assert!(
        binary.is_absolute() && binary.is_file(),
        "Select an absolute Squint testkit executable"
    );
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir(&tool).unwrap();
    let config = tool.join("config.toml");
    let script = tool.join("held.json");
    std::fs::write(&config, include_str!("../testkit/squint/config.toml")).unwrap();
    std::fs::write(&script, include_str!("../testkit/squint/held.json")).unwrap();
    let input = dir.path().join("original.png");
    let bytes = include_bytes!("../ui/tests/fixtures/sample.png");
    std::fs::write(&input, bytes).unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let image = assets.import_path(input.clone()).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            assets,
            vec![Descriptor {
                config: None,
                id: "squint".into(),
                executable: binary,
                args: vec![
                    "serve".into(),
                    "--config".into(),
                    config.display().to_string(),
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
    let description = match c.description("squint").await {
        Ok(value) => value,
        Err(error) => {
            let snapshot = c.snapshot().await.unwrap();
            assert!(c.close().await);
            c.assets.database.shutdown().await.unwrap();
            panic!("{error}; {snapshot}");
        }
    };
    assert_eq!(description.document["contract"], "0.3.0");
    for action in description.document["actions"].as_array().unwrap() {
        assert_eq!(action["cost"]["may_spend"], false, "{action}");
        assert_eq!(action["readiness"]["state"], "ready", "{action}");
    }
    let id = "squint-probe";
    let request = serde_json::from_value(json!({
        "id":id, "tool":"squint", "action":action,
        "revision":description.revision, "params":params,
        "images":{"candidate":[image.id],"source":[image.id]}
    }))
    .unwrap();
    c.submit(request).await.unwrap();
    let saved = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let attempt = c.attempts.get(id).await.unwrap();
            if attempt.preservation["contract"] != "pending" {
                break attempt;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await;
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
    let saved = saved.unwrap();
    assert_eq!(saved.execution, "completed", "{:?}", saved.notices);
    assert_eq!(
        saved.preservation["contract"], "complete",
        "{:?}",
        saved.notices
    );
    assert!(saved.wire.is_some());
    let result = saved.result.unwrap();
    assert_eq!(result["spend"]["usd"], 0.0);
    if action == "estimate" {
        assert!(result["body"]["envelope_usd"].as_f64().unwrap() > 0.0);
    } else {
        assert_eq!(result["body"]["verdict"], "pass");
    }
    assert!(saved.outputs.is_empty());
    assert_eq!(
        saved.inputs["candidate"][0]["asset"]["sha256"],
        saved.inputs["source"][0]["asset"]["sha256"]
    );
    assert_eq!(std::fs::read(input).unwrap(), bytes);
}
