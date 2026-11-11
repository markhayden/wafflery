//! The reserved install and uninstall controls (table-manners 0.6 draft, 5.3) against sample-lab's
//! scripted `installables` scenario: an install outlives its HTTP request, the description reports
//! its progress and outcome, uninstall of an installing model cancels it, and nothing is a run.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tower::ServiceExt;
use wafflery::{
    host::{api, auth::Auth},
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};

struct Harness {
    root: tempfile::TempDir,
    coordinator: Arc<Coordinator>,
    app: Router,
    token: String,
}
impl Harness {
    async fn new() -> Self {
        static BINARY: OnceLock<PathBuf> = OnceLock::new();
        let binary = BINARY.get_or_init(|| {
            assert!(
                Command::new(env!("CARGO"))
                    .args(["build", "--locked", "--example", "sample-lab"])
                    .status()
                    .unwrap()
                    .success()
            );
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
        });
        let root = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&root.path().join("Library")).unwrap();
        let coordinator = Arc::new(
            Coordinator::open(
                assets.clone(),
                vec![Descriptor {
                    config: None,
                    id: "fixture".into(),
                    executable: binary.clone(),
                    args: vec![
                        root.path().join("tool").display().to_string(),
                        "installables".into(),
                    ],
                    cwd: root.path().into(),
                    allow_dirs: vec![root.path().into()],
                    evidence_dirs: vec![root.path().into()],
                }],
            )
            .await
            .unwrap(),
        );
        let auth = Auth::new().unwrap();
        let token = auth
            .exchange(&auth.issue().unwrap())
            .unwrap()
            .unwrap()
            .token;
        let (quit, _) = tokio::sync::watch::channel(false);
        let app = api::router_with_tools(
            "http://127.0.0.1:45678",
            auth,
            quit,
            assets,
            Some(coordinator.clone()),
        );
        Self {
            root,
            coordinator,
            app,
            token,
        }
    }
    async fn post(&self, path: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", self.token))
            .body(Body::from("{}"))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn installable(&self, id: &str) -> Value {
        let declaration = self.coordinator.refresh("fixture").await.unwrap();
        declaration.document["installables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == id)
            .unwrap()
            .clone()
    }
    async fn until(&self, id: &str, check: impl Fn(&Value) -> bool) -> Value {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let item = self.installable(id).await;
                if check(&item) {
                    return item;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the installable never reached the expected state")
    }
    async fn close(&self) {
        assert!(self.coordinator.close().await);
        self.coordinator.assets.database.shutdown().await.unwrap();
    }
}

/// The first action's requirement for the `large` model choice.
fn large_requirement(document: &Value) -> Value {
    document["actions"][0]["readiness"]["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "model:large")
        .unwrap()
        .clone()
}

#[tokio::test]
async fn an_install_outlives_its_request_and_the_description_reports_it() {
    let host = Harness::new().await;
    let before = host.installable("large").await;
    let (status, started) = host
        .post("/api/tools/fixture/installables/large/install")
        .await;
    let installing = host
        .until("large", |item| item["state"] == "installing")
        .await;
    let declaration = host.coordinator.description("fixture").await.unwrap();
    let readiness_before = large_requirement(&declaration.document)["state"].clone();
    std::fs::write(host.root.path().join("tool/release-install"), "").unwrap();
    let installed = host
        .until("large", |item| item["state"] == "installed")
        .await;
    let declaration = host.coordinator.description("fixture").await.unwrap();
    let attempts = host.coordinator.snapshot().await.unwrap()["attempts"].clone();
    host.close().await;
    assert_eq!(before["state"], "not_installed");
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(started["state"], "installing");
    assert_eq!(installing["progress"], 0.25);
    assert_eq!(readiness_before, "missing");
    assert_eq!(installed["last_outcome"]["action"], "install");
    assert_eq!(installed["last_outcome"]["state"], "succeeded");
    assert_eq!(large_requirement(&declaration.document)["state"], "present");
    // A control is never a run: nothing is saved to history.
    assert_eq!(attempts.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn uninstall_cancels_an_install_in_progress_and_removes_an_installed_model() {
    let host = Harness::new().await;
    host.post("/api/tools/fixture/installables/large/install")
        .await;
    host.until("large", |item| item["state"] == "installing")
        .await;
    let (cancel_status, cancel) = host
        .post("/api/tools/fixture/installables/large/uninstall")
        .await;
    let cancelled = host
        .until("large", |item| item["last_outcome"]["state"] == "cancelled")
        .await;
    let (remove_status, removed) = host
        .post("/api/tools/fixture/installables/small/uninstall")
        .await;
    let small = host.installable("small").await;
    let declaration = host.coordinator.description("fixture").await.unwrap();
    host.close().await;
    assert_eq!(cancel_status, StatusCode::OK);
    assert_eq!(cancel["id"], "large");
    assert_eq!(cancelled["state"], "not_installed");
    assert_eq!(remove_status, StatusCode::OK);
    assert_eq!(removed["state"], "not_installed");
    assert_eq!(small["last_outcome"]["action"], "uninstall");
    // The default choice is now missing, so the action needs setup.
    assert_eq!(
        declaration.document["actions"][0]["readiness"]["state"],
        "needs_setup"
    );
}

#[tokio::test]
async fn an_unlisted_installable_or_control_is_refused_before_any_call() {
    let host = Harness::new().await;
    let (unknown, _) = host
        .post("/api/tools/fixture/installables/giant/install")
        .await;
    let (control, _) = host
        .post("/api/tools/fixture/installables/large/upgrade")
        .await;
    let called = host.root.path().join("tool/install-calls").exists();
    host.close().await;
    assert_eq!(unknown, StatusCode::NOT_FOUND);
    assert_eq!(control, StatusCode::NOT_FOUND);
    assert!(!called);
}
