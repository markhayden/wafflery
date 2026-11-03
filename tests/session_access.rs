use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Duration,
};
use wafflery::{
    library::{assets::AssetStore, attempts::Submission},
    tools::{coordinator::Coordinator, registration::Descriptor},
};

struct Host {
    _temp: tempfile::TempDir,
    c: Arc<Coordinator>,
    tool: PathBuf,
    requested: PathBuf,
    narrow: PathBuf,
    execution: PathBuf,
    config: PathBuf,
    image: String,
}
impl Host {
    async fn new() -> Self {
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
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let tool = root.join("tool");
        let requested = root.join("inputs");
        let narrow = requested.join("selected");
        fs::create_dir_all(&narrow).unwrap();
        fs::create_dir(&tool).unwrap();
        let assets = AssetStore::open(&root.join("Library")).unwrap();
        let execution = assets.root().join("execution");
        let original = root.join("source.png");
        fs::write(&original, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let image = assets.import_path(original).await.unwrap().id;
        let config = root.join("access.json");
        fs::write(
            &config,
            json!({"allowed_dirs":[narrow,execution]}).to_string(),
        )
        .unwrap();
        let c = Arc::new(
            Coordinator::open(
                assets,
                vec![Descriptor {
                    id: "fixture".into(),
                    executable: binary.clone(),
                    args: vec![tool.display().to_string(), "estimates".into()],
                    cwd: root.clone(),
                    allow_dirs: vec![requested.clone()],
                    evidence_dirs: vec![root],
                    config: Some(config.clone()),
                }],
            )
            .await
            .unwrap(),
        );
        Self {
            _temp: temp,
            c,
            tool,
            requested,
            narrow,
            execution,
            config,
            image,
        }
    }
    async fn request(&self, id: &str, label: &str) -> Submission {
        let d = self.c.description("fixture").await.unwrap();
        serde_json::from_value(
            json!({"id":id,"tool":"fixture","action":"price_test","revision":d.revision,
            "params":{"label":label,"max_spend_usd":0.5},"images":{"source":[self.image]}}),
        )
        .unwrap()
    }
    fn configure(&self, dirs: &[PathBuf]) {
        fs::write(&self.config, json!({"allowed_dirs":dirs}).to_string()).unwrap();
    }
    async fn snapshot_session(&self) -> Value {
        self.c.snapshot().await.unwrap()["tools"][0]["declaration"]["tool_session"].clone()
    }
    async fn idle(&self) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while self.c.snapshot().await.unwrap()["tools"][0]["busy"] == true {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("attempt worker did not finish cleanup");
    }
    async fn finish(self) {
        let (quit, mut stopped) = tokio::sync::watch::channel(false);
        self.c.begin_shutdown(true, quit).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), stopped.wait_for(|done| *done))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(self.c.shutdown_status().await.phase, "stopped");
        self.c.assets.database.shutdown().await.unwrap();
    }
}
fn dirs(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}
async fn wait_for(path: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("peer did not reach the explicit gate");
}

#[tokio::test]
async fn configured_grants_and_identity_stay_frozen_until_explicit_restart() {
    let h = Host::new().await;
    let before = h.c.session("fixture").await.unwrap();
    assert_eq!(
        dirs(&before["allowed_dirs"]),
        dirs(&json!([h.narrow, h.execution]))
    );
    let snapshot = h.c.snapshot().await.unwrap();
    assert_eq!(
        dirs(&snapshot["tools"][0]["directory_grants"]["requested_dirs"]),
        dirs(&json!([h.requested, h.execution]))
    );
    let request = h.request("frozen", "frozen configuration").await;
    h.c.prepare(request.clone()).await.unwrap();
    let first = h.c.estimate_prepared(&request.id).await.unwrap().estimate;
    h.configure(&[]);
    h.c.refresh("fixture").await.unwrap();
    assert_eq!(h.c.session("fixture").await.unwrap(), before);
    let unchanged = h.c.estimate_prepared(&request.id).await.unwrap().estimate;
    assert_eq!(
        unchanged["effective"]["config_identity"],
        first["effective"]["config_identity"]
    );
    let replacement = h.c.restart("fixture", true).await.unwrap();
    assert_ne!(replacement.tool_session["id"], before["id"]);
    assert_eq!(replacement.tool_session["allowed_dirs"], json!([]));
    fs::remove_file(h.tool.join("estimate-read")).unwrap();
    let denied = h.request("denied", "denied copied input").await;
    h.c.prepare(denied.clone()).await.unwrap();
    let error =
        h.c.estimate_prepared(&denied.id)
            .await
            .err()
            .expect("estimate must be refused")
            .to_string();
    assert!(error.contains("outside input grants"), "{error}");
    assert!(!h.tool.join("estimate-read").exists());
    assert!(!h.tool.join("calls.ndjson").exists());
    assert!(h.c.attempts.get(&denied.id).await.is_err());
    assert_eq!(h.c.session("fixture").await.unwrap()["spent_usd"], 0);
    h.finish().await;
}

#[tokio::test]
async fn invalid_resource_and_estimate_grants_never_replace_the_last_valid_observation() {
    let h = Host::new().await;
    let before = h.c.session("fixture").await.unwrap();
    let resource = h.tool.join("resource-grants.json");
    fs::write(&resource, "[]").unwrap();
    let error = h.c.session("fixture").await.unwrap_err().to_string();
    assert!(error.contains("effective access"), "{error}");
    assert_eq!(h.snapshot_session().await, before);
    fs::remove_file(resource).unwrap();
    let request = h.request("hostile-quote", "ledger").await;
    h.c.prepare(request.clone()).await.unwrap();
    let estimate = h.tool.join("estimate-grants.json");
    fs::write(&estimate, "[]").unwrap();
    let error =
        h.c.estimate_prepared(&request.id)
            .await
            .err()
            .expect("estimate must be refused")
            .to_string();
    assert!(error.contains("effective access"), "{error}");
    assert_eq!(h.snapshot_session().await, before);
    assert!(h.c.attempts.get(&request.id).await.is_err());
    fs::remove_file(estimate).unwrap();
    let valid = h.c.session("fixture").await.unwrap();
    assert_eq!(valid["spent_usd"], 0.08);
    assert_eq!(valid["allowed_dirs"], before["allowed_dirs"]);
    assert_eq!(h.snapshot_session().await, valid);
    h.finish().await;
}

#[tokio::test]
async fn disappearing_grant_keeps_final_spend_and_restart_keeps_old_evidence() {
    let h = Host::new().await;
    let before = h.c.session("fixture").await.unwrap();
    let request = h.request("held", "paid-held").await;
    h.c.prepare(request.clone()).await.unwrap();
    assert_eq!(
        h.c.estimate_prepared(&request.id).await.unwrap().estimate["allowed"],
        true
    );
    h.c.submit_prepared(request.clone()).await.unwrap();
    wait_for(&h.tool.join("paid-provider-started")).await;
    fs::remove_dir(&h.narrow).unwrap();
    let reserved = h.c.session("fixture").await.unwrap();
    assert_eq!(reserved["reserved_usd"], 0.12);
    assert_eq!(reserved["allowed_dirs"], before["allowed_dirs"]);
    fs::write(h.tool.join("release-paid"), "release").unwrap();
    h.idle().await;
    let saved = h.c.attempts.get(&request.id).await.unwrap();
    assert_eq!(saved.execution, "completed");
    assert_eq!(
        saved.preservation["contract"], "complete",
        "{:?}",
        saved.notices
    );
    assert_eq!(saved.observations["session_after"]["spent_usd"], 0.08);
    assert_eq!(
        saved.observations["session_after"]["reserved_usd"].as_f64(),
        Some(0.0)
    );
    assert_eq!(saved.observations["session_after"]["id"], before["id"]);
    let original = serde_json::to_vec(&saved).unwrap();
    h.configure(std::slice::from_ref(&h.execution));
    let replacement = h.c.restart("fixture", true).await.unwrap();
    assert_ne!(replacement.tool_session["id"], before["id"]);
    assert_eq!(replacement.tool_session["spent_usd"], 0);
    assert_eq!(
        replacement.tool_session["allowed_dirs"],
        json!([h.execution])
    );
    assert_eq!(
        serde_json::to_vec(&h.c.attempts.get(&request.id).await.unwrap()).unwrap(),
        original
    );
    h.finish().await;
}
