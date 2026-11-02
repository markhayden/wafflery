use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Duration,
};
use wafflery::{
    library::{assets::AssetStore, attempts::Submission},
    tools::{coordinator::Coordinator, registration::Descriptor},
};

async fn setup(label: &str) -> (tempfile::TempDir, Arc<Coordinator>, Submission) {
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
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = dir.path().join("original.png");
    std::fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(input).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            assets,
            vec![Descriptor {
                id: "fixture".into(),
                config: None,
                executable: binary.clone(),
                args: vec![
                    dir.path().join("runs").display().to_string(),
                    "estimates".into(),
                ],
                cwd: dir.path().to_owned(),
                roots: vec![dir.path().to_owned()],
            }],
        )
        .await
        .unwrap(),
    );
    let declaration = c.description("fixture").await.unwrap();
    let request = serde_json::from_value(json!({"id":"prepared","tool":"fixture","action":"price_test","revision":declaration.revision,"params":{"label":label,"max_spend_usd":0.5},"images":{"source":[asset.id]}})).unwrap();
    (dir, c, request)
}
async fn marker(path: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("peer did not reach the expected gate");
}
async fn finish(c: &Arc<Coordinator>) {
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), stopped.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
    c.assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn estimates_use_exact_prepared_inputs_while_a_free_run_holds_execution() {
    let (dir, c, request) = setup("normal").await;
    let mut free = request.clone();
    free.id = "running".into();
    free.action = "mark_band".into();
    free.params.as_object_mut().unwrap().remove("max_spend_usd");
    c.submit(free).await.unwrap();
    marker(&dir.path().join("runs/calls.ndjson")).await;
    c.prepare(request.clone()).await.unwrap();
    std::fs::write(
        dir.path().join("original.png"),
        b"changed after preparation",
    )
    .unwrap();
    let quote = c.estimate_prepared(&request.id).await;
    let snapshot = c.snapshot().await.unwrap();
    let recorded: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("runs/estimate-started")).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("runs/release-run"), b"").unwrap();
    finish(&c).await;
    assert!(quote.unwrap().observation()["allowed"].as_bool().unwrap());
    assert_eq!(snapshot["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["tools"][0]["busy"], true);
    assert_eq!(
        recorded["source"]["expect"]["file_sha256"],
        std::fs::read_to_string(dir.path().join("runs/estimate-read")).unwrap()
    );
    assert!(
        recorded["source"]["path"]
            .as_str()
            .unwrap()
            .contains("execution/prepared/")
    );
}
#[tokio::test]
async fn abandoned_request_and_discard_keep_remote_input_lease_and_bound_concurrency() {
    let (dir, c, request) = setup("hold").await;
    c.prepare(request.clone()).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    assert!(c.estimate_prepared("prepared").await.is_err());
    task.abort();
    c.discard_preparation("prepared").await.unwrap();
    let path = c.assets.root().join("execution/prepared");
    assert!(path.exists());
    std::fs::write(dir.path().join("runs/release-estimate"), b"").unwrap();
    marker(&dir.path().join("runs/estimate-read")).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(c.preparation("prepared").await.is_err());
    assert!(
        c.snapshot().await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/estimates.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    finish(&c).await;
}
#[tokio::test]
async fn timed_out_estimate_keeps_reader_alive_without_retries_or_late_quote() {
    let (dir, c, request) = setup("hold").await;
    c.prepare(request).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    let result = tokio::time::timeout(Duration::from_secs(35), task)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    assert!(
        c.session("fixture").await.is_ok(),
        "timeout releases ledger observation lock"
    );
    assert!(
        c.estimate_prepared("prepared").await.is_err(),
        "old remote reader still owns the estimate slot"
    );
    c.discard_preparation("prepared").await.unwrap();
    let path = c.assets.root().join("execution/prepared");
    assert!(path.exists());
    std::fs::write(dir.path().join("runs/release-estimate"), b"").unwrap();
    marker(&dir.path().join("runs/estimate-read")).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(c.preparation("prepared").await.is_err());
    assert!(
        c.snapshot().await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    finish(&c).await;
}
#[tokio::test]
async fn malformed_quotes_and_rpc_refusals_create_no_attempt() {
    for label in ["wrong-session", "plain-error", "structured-error"] {
        let (_dir, c, request) = setup(label).await;
        c.prepare(request).await.unwrap();
        assert!(c.estimate_prepared("prepared").await.is_err(), "{label}");
        assert!(
            c.snapshot().await.unwrap()["attempts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        finish(&c).await;
    }
}

#[tokio::test]
async fn shutdown_closes_the_reader_before_joining_estimate_workers() {
    let (dir, c, request) = setup("hold").await;
    c.prepare(request).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    let path = c.assets.root().join("execution/prepared");
    assert!(path.exists());
    finish(&c).await;
    assert!(task.await.unwrap().is_err());
    assert!(!path.exists());
    assert!(!dir.path().join("runs/estimate-read").exists());
}

#[tokio::test]
async fn broken_transport_keeps_inputs_until_the_owned_child_is_confirmed_stopped() {
    let (dir, c, request) = setup("disconnect").await;
    std::fs::write(dir.path().join("runs/hold-after-eof"), b"").unwrap();
    c.prepare(request).await.unwrap();
    let task = tokio::spawn({
        let c = c.clone();
        async move { c.estimate_prepared("prepared").await }
    });
    marker(&dir.path().join("runs/estimate-started")).await;
    c.discard_preparation("prepared").await.unwrap();
    let path = c.assets.root().join("execution/prepared");
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        while c.shutdown_status().await.phase != "waiting" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        path.exists(),
        "transport loss and EOF are not confirmation of process exit"
    );
    c.force_shutdown(quit).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), stopped.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(!path.exists());
    c.assets.database.shutdown().await.unwrap();
}
