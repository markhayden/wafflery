use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use wafflehouse::tools::{client::ToolClient, registration::Descriptor};
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
        id: "unrelated-registration-name".into(),
        executable: binary(),
        args: vec![root.join("runs").display().to_string(), scenario.into()],
        cwd: root.to_owned(),
        roots: vec![root.to_owned()],
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
fn rejects_missing_or_relative_executable_and_relative_roots() {
    let dir = tempfile::tempdir().unwrap();
    let mut registration = descriptor(dir.path(), "normal");
    registration.executable = dir.path().join("missing");
    assert!(ToolClient::spawn(registration.clone()).is_err());
    registration.executable = "echo".into();
    assert!(ToolClient::spawn(registration.clone()).is_err());
    registration.executable = binary();
    registration.roots = vec![".".into()];
    assert!(ToolClient::spawn(registration).is_err());
}
