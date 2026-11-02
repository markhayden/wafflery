use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tower::ServiceExt;
use wafflery::{
    host::{api, auth::Auth},
    library::{
        assets::AssetStore,
        attempts::{Attempt, Submission},
    },
    tools::{coordinator::Coordinator, registration::Descriptor},
};
struct Harness {
    root: tempfile::TempDir,
    coordinator: Arc<Coordinator>,
    app: Router,
    token: String,
    image: String,
}
impl Harness {
    async fn new(scenario: &str) -> Self {
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
        let source = root.path().join("source.png");
        std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let image = assets.import_path(source).await.unwrap().id;
        let coordinator = Arc::new(
            Coordinator::open(
                assets.clone(),
                vec![Descriptor {
                    config: None,
                    id: "fixture".into(),
                    executable: binary.clone(),
                    args: vec![
                        root.path().join("tool").display().to_string(),
                        scenario.into(),
                    ],
                    cwd: root.path().into(),
                    roots: vec![root.path().into()],
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
            image,
        }
    }
    async fn refresh(&self, authenticated: bool) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/tools/fixture/refresh")
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("content-type", "application/json");
        if authenticated {
            request = request.header("authorization", format!("Bearer {}", self.token));
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from("{}")).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn request(&self, id: &str, filler: bool) -> Submission {
        let declaration = self.coordinator.description("fixture").await.unwrap();
        serde_json::from_value(json!({"id":id,"tool":"fixture","action":"mark_band","revision":declaration.revision,"params":{"label":"Readiness test","filler":filler},"images":{"source":[self.image]}})).unwrap()
    }
    async fn terminal(&self, id: &str) -> Attempt {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let attempt = self.coordinator.attempts.get(id).await.unwrap();
                if !matches!(attempt.execution.as_str(), "submitted" | "running")
                    && attempt.preservation["contract"] != "pending"
                    && self.coordinator.snapshot().await.unwrap()["tools"][0]["busy"] == false
                {
                    return attempt;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn close(&self) {
        assert!(self.coordinator.close().await);
        self.coordinator.assets.database.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn description_refresh_is_authenticated_and_available_during_execution() {
    let host = Harness::new("readiness-held").await;
    let unauthorized = host.refresh(false).await.0;
    let before = host.coordinator.description("fixture").await.unwrap();
    host.coordinator
        .submit(host.request("held", false).await)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !host.root.path().join("tool/calls.ndjson").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    std::fs::write(host.root.path().join("tool/credential-state"), "valid").unwrap();
    let (status, refreshed) = tokio::time::timeout(Duration::from_secs(2), host.refresh(true))
        .await
        .unwrap();
    let while_running = host.coordinator.attempts.get("held").await.unwrap();
    std::fs::write(host.root.path().join("tool/release-run"), "").unwrap();
    let completed = host.terminal("held").await;
    let snapshot = host.coordinator.snapshot().await.unwrap();
    host.close().await;
    assert_eq!(unauthorized, StatusCode::UNAUTHORIZED);
    assert_eq!(status, StatusCode::OK);
    assert_ne!(refreshed["revision"], before.revision);
    assert_eq!(
        refreshed["document"]["actions"][1]["readiness"]["requirements"][0]["state"],
        "valid"
    );
    assert_eq!(while_running.execution, "running");
    assert_eq!(completed.execution, "completed");
    assert_eq!(
        completed.declaration["document"]["actions"][1]["readiness"]["requirements"][0]["state"],
        "present"
    );
    assert_eq!(
        snapshot["tools"][0]["declaration"]["revision"],
        refreshed["revision"]
    );
    assert_eq!(snapshot["attempts"].as_array().unwrap().len(), 1);
}
#[tokio::test]
async fn a_not_ready_call_has_no_run_and_does_not_block_a_corrected_free_attempt() {
    let host = Harness::new("readiness").await;
    host.coordinator
        .submit(host.request("needs-setup", true).await)
        .await
        .unwrap();
    let refused = host.terminal("needs-setup").await;
    let no_run = !host.root.path().join("tool/calls.ndjson").exists();
    let retry = host
        .coordinator
        .submit(host.request("corrected", false).await)
        .await;
    let corrected = if retry.is_ok() {
        Some(host.terminal("corrected").await)
    } else {
        None
    };
    let session = host.coordinator.session("fixture").await.unwrap();
    host.close().await;
    assert!(no_run);
    assert_eq!(refused.execution, "rejected");
    assert!(refused.result.is_none());
    assert_eq!(
        refused.raw_response.unwrap()["error"]["data"]["reason"],
        "not_ready"
    );
    assert_eq!(
        refused.observations["refusal"]["requirements"][0]["state"],
        "missing"
    );
    assert!(
        !refused
            .notices
            .iter()
            .any(|notice| notice.contains("credential error"))
    );
    assert!(retry.is_ok(), "{retry:?}");
    assert_eq!(corrected.unwrap().execution, "completed");
    assert_eq!(session["spent_usd"], 0);
}

#[tokio::test]
async fn definitive_rpc_refusals_preserve_zero_spend_and_allow_fresh_corrected_attempts() {
    for label in ["refuse plain", "refuse structured", "refuse internal"] {
        let host = Harness::new("refusals").await;
        let mut request = host.request("refused", false).await;
        request.params = json!({"label":label});
        host.coordinator.submit(request.clone()).await.unwrap();
        // Read the state before teardown even on a failure, so the peer is always reaped.
        let refused = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let attempt = host.coordinator.attempts.get("refused").await.unwrap();
                if attempt.preservation["contract"] != "pending" {
                    return attempt;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let no_run = !host.root.path().join("tool/calls.ndjson").exists();
        let mut corrected = request.clone();
        corrected.id = "corrected".into();
        corrected.params["label"] = json!("Corrected input");
        // Availability follows input cleanup, not just publication of the refusal.
        let available = tokio::time::timeout(Duration::from_secs(2), async {
            while host.coordinator.snapshot().await.unwrap()["tools"][0]["busy"] == true {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok();
        let completed = if available {
            host.coordinator.submit(corrected).await.unwrap();
            Some(host.terminal("corrected").await)
        } else {
            None
        };
        let original = host.coordinator.submit(request).await.unwrap();
        let count = std::fs::read_to_string(host.root.path().join("tool/calls.ndjson"))
            .unwrap_or_default()
            .lines()
            .count();
        host.close().await;
        assert_eq!(
            refused.execution, "rejected",
            "{label}: {:?}",
            refused.notices
        );
        assert!(no_run);
        assert!(refused.result.is_none());
        assert_eq!(refused.preservation["contract"], "not_applicable");
        assert_eq!(refused.observations["refusal"]["run"], Value::Null);
        assert_eq!(refused.observations["spent_usd"], 0);
        assert!(refused.raw_response.as_ref().unwrap()["error"]["code"].is_number());
        let completed = completed.expect("definitive refusal releases the host slot");
        assert_eq!(completed.execution, "completed");
        assert_eq!(completed.id, completed.request.id);
        assert_eq!(
            completed.wire.as_ref().unwrap()["params"]["_meta"]["table-manners/client_run_id"],
            completed.id
        );
        assert_eq!(
            original.execution, "rejected",
            "idempotent HTTP retry must not redispatch"
        );
        assert_eq!(count, 1);
    }
}
