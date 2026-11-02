use super::*;
use crate::tools::registration::Descriptor;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

async fn client() -> (tempfile::TempDir, Arc<ToolClient>, Declaration, Value) {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    let binary = BINARY.get_or_init(|| {
        assert!(
            std::process::Command::new(env!("CARGO"))
                .args(["build", "--locked", "--example", "sample-lab"])
                .status()
                .unwrap()
                .success()
        );
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.png");
    std::fs::write(
        &path,
        include_bytes!("../../../ui/tests/fixtures/sample.png"),
    )
    .unwrap();
    let mut client = ToolClient::spawn(Descriptor {
        id: "fixture".into(),
        config: None,
        executable: binary.clone(),
        args: vec![dir.path().display().to_string(), "estimates".into()],
        cwd: dir.path().to_owned(),
        roots: vec![dir.path().to_owned()],
    })
    .unwrap();
    client.initialize().await.unwrap();
    let declaration = client.describe().await.unwrap();
    let arguments = json!({"label":"hold","max_spend_usd":0.5,"source":{"path":path}});
    (dir, Arc::new(client), declaration, arguments)
}
#[tokio::test]
async fn deadline_spent_waiting_for_ledger_never_dispatches_a_control() {
    let (dir, client, declaration, arguments) = client().await;
    let guard = client.observed_session.lock().await;
    let result = client
        .estimate_control(
            &declaration,
            "price_test",
            &arguments,
            0.5,
            Instant::now() + Duration::from_millis(20),
        )
        .await;
    drop(guard);
    assert!(client.close().await.unwrap());
    assert!(matches!(result, Err(Error::Timeout(_))));
    assert!(!dir.path().join("estimates.ndjson").exists());
}
#[tokio::test]
async fn late_response_cannot_overwrite_a_newer_session_observation() {
    let (dir, client, declaration, arguments) = client().await;
    let deadline = Instant::now() + Duration::from_secs(30);
    let task = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .estimate_control(&declaration, "price_test", &arguments, 0.5, deadline)
                .await
        }
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !dir.path().join("estimate-started").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    // The external peer is reading now. Only then advance the host deadline.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::time::resume();
    let mut guard = tokio::time::timeout(Duration::from_secs(1), client.observed_session.lock())
        .await
        .unwrap();
    guard.as_mut().unwrap()["spent_usd"] = json!(0.33);
    drop(guard);
    assert!(
        !task.is_finished(),
        "deadline must not abandon the remote reader"
    );
    std::fs::write(dir.path().join("release-estimate"), b"").unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_eq!(
        client.observed_session.lock().await.as_ref().unwrap()["spent_usd"],
        json!(0.33)
    );
    assert!(client.close().await.unwrap());
}
