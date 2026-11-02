use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use wafflery::tools::{client::ToolClient, registration::Descriptor};
use wafflery::{library::assets::AssetStore, tools::coordinator::Coordinator};
fn binary() -> PathBuf {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY
        .get_or_init(|| {
            assert!(
                Command::new(env!("CARGO"))
                    .args(["build", "--locked", "--example", "sample-lab"])
                    .status()
                    .unwrap()
                    .success()
            );
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
        })
        .clone()
}
fn descriptor(root: &Path, scenario: &str) -> Descriptor {
    Descriptor {
        config: None,
        id: "unrelated-registration-name".into(),
        executable: binary(),
        args: vec![root.join("runs").display().to_string(), scenario.into()],
        cwd: root.to_owned(),
        allow_dirs: vec![root.to_owned()],
        evidence_dirs: vec![root.to_owned()],
    }
}
#[tokio::test]
async fn connects_arbitrary_peer_pages_declarations_and_closes_owned_child() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = ToolClient::spawn(descriptor(dir.path(), "normal")).unwrap();
    client.initialize().await.unwrap();
    let declaration = client.describe().await.unwrap();
    assert_eq!(declaration.document["tool"]["name"], "sample-lab");
    assert!(declaration.schemas.contains_key("mark_band"));
    let runs = client.read_resource("sample-lab://runs").await.unwrap();
    assert!(
        runs["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("runs")
    );
    assert!(
        client.close().await.unwrap(),
        "EOF must stop the independent child"
    );
}
#[tokio::test]
async fn refuses_wrong_protocol_contract_and_malformed_declarations_without_execution() {
    for scenario in [
        "wrong-protocol",
        "unsupported-contract",
        "malformed",
        "bad-schema",
        "missing-cost",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut client = ToolClient::spawn(descriptor(dir.path(), scenario)).unwrap();
        let initialized = client.initialize().await;
        assert!(
            initialized.is_err() || client.describe().await.is_err(),
            "{scenario}"
        );
        assert!(!dir.path().join("runs/calls.ndjson").exists());
        assert!(client.close().await.unwrap(), "{scenario}");
    }
}
#[test]
fn rejects_missing_or_relative_executable_and_relative_input_grants() {
    let dir = tempfile::tempdir().unwrap();
    let mut registration = descriptor(dir.path(), "normal");
    registration.executable = dir.path().join("missing");
    assert!(ToolClient::spawn(registration.clone()).is_err());
    registration.executable = "echo".into();
    assert!(ToolClient::spawn(registration.clone()).is_err());
    registration.executable = binary();
    registration.allow_dirs = vec![".".into()];
    assert!(ToolClient::spawn(registration).is_err());
}

#[tokio::test]
async fn invalid_schema_names_its_tool_and_does_not_block_another_registration() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = dir.path().join("source.png");
    std::fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = store.import_path(input).await.unwrap();
    let mut registrations = Vec::new();
    for (id, scenario) in [("broken", "misplaced-definitions"), ("working", "normal")] {
        let root = dir.path().join(id);
        std::fs::create_dir(&root).unwrap();
        let mut entry = descriptor(&root, scenario);
        entry.id = id.into();
        registrations.push(entry);
    }
    let coordinator = std::sync::Arc::new(Coordinator::open(store, registrations).await.unwrap());
    let snapshot = coordinator.snapshot().await.unwrap();
    let broken = snapshot["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["id"] == "broken")
        .unwrap();
    let error = broken["error"].as_str().unwrap();
    let declaration = coordinator.description("working").await.unwrap();
    let request = serde_json::from_value(json!({
        "id":"working-attempt", "tool":"working", "action":"mark_band",
        "revision":declaration.revision, "params":{"label":"still available"},
        "images":{"source":[asset.id]}
    }))
    .unwrap();
    coordinator.submit(request).await.unwrap();
    let saved = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let saved = coordinator.attempts.get("working-attempt").await.unwrap();
            if saved.preservation["contract"] != "pending" {
                break saved;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(coordinator.close().await);
    coordinator.assets.database.shutdown().await.unwrap();
    assert!(
        error.contains("MCP tool 'mark_band' input schema"),
        "{error}"
    );
    assert!(error.contains("/$defs/Kind"), "{error}");
    assert!(broken["declaration"].is_null());
    assert_eq!(saved.execution, "completed");
    assert_eq!(saved.preservation["contract"], "complete");
    assert!(!dir.path().join("broken/runs/calls.ndjson").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("working/runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[tokio::test]
async fn control_discovery_accepts_unconfigured_tools_without_exposing_reserved_controls_as_actions()
 {
    for (scenario, name) in [
        ("control-needs-setup", "sample-lab"),
        ("control-lens", "test-lens"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut client = ToolClient::spawn(descriptor(dir.path(), scenario)).unwrap();
        client.initialize().await.unwrap();
        let declaration = client.describe().await;
        assert!(client.close().await.unwrap());
        let declaration = declaration.unwrap();
        assert_eq!(declaration.document["contract"], "0.4.0");
        assert_eq!(declaration.document["tool"]["name"], name);
        assert!(declaration.schemas.contains_key("estimate"));
        assert!(
            declaration.document["actions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|action| action["name"] != "estimate" && action["name"] != "describe")
        );
        assert!(!dir.path().join("runs/calls.ndjson").exists());
    }
}

#[tokio::test]
async fn control_discovery_rejects_legacy_live_contract_and_paid_declarations_missing_controls_or_caps()
 {
    for (scenario, reason) in [
        ("control-legacy", "exactly 0.4.0"),
        ("control-no-estimate", "estimate"),
        ("control-no-cap", "max_spend_usd"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut client = ToolClient::spawn(descriptor(dir.path(), scenario)).unwrap();
        client.initialize().await.unwrap();
        let result = client.describe().await;
        assert!(client.close().await.unwrap());
        let error = match result {
            Err(error) => error.to_string(),
            Ok(_) => panic!("accepted {scenario}"),
        };
        assert!(error.contains(reason), "{scenario}: {error}");
        assert!(!dir.path().join("runs/calls.ndjson").exists());
    }
}
