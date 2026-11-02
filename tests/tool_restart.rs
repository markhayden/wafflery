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
    library::{assets::AssetStore, attempts::Submission},
    tools::{coordinator::Coordinator, registration::Descriptor},
};

struct Host {
    root: tempfile::TempDir,
    c: Arc<Coordinator>,
    app: Router,
    token: String,
    image: String,
}
impl Host {
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
        let tool = root.path().join("tool");
        std::fs::create_dir(&tool).unwrap();
        std::fs::write(tool.join("scenario"), scenario).unwrap();
        let script = root.path().join("peer.sh");
        std::fs::write(
            &script,
            r#"fixture_bin=$1
fixture_root=$2
shift 2
exec "$fixture_bin" "$fixture_root" "$(cat "$fixture_root/scenario")" "$@"
"#,
        )
        .unwrap();
        let assets = AssetStore::open(&root.path().join("Library")).unwrap();
        let image = root.path().join("source.png");
        std::fs::write(&image, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let image = assets.import_path(image).await.unwrap().id;
        let c = Arc::new(
            Coordinator::open(
                assets.clone(),
                vec![Descriptor {
                    id: "fixture".into(),
                    executable: "/bin/sh".into(),
                    args: vec![
                        script.display().to_string(),
                        binary.display().to_string(),
                        tool.display().to_string(),
                    ],
                    cwd: root.path().into(),
                    allow_dirs: vec![tool.clone()],
                    evidence_dirs: vec![tool],
                    config: None,
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
            Some(c.clone()),
        );
        Self {
            root,
            c,
            app,
            token,
            image,
        }
    }
    async fn restart(&self, authenticated: bool, confirmed: bool) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/tools/fixture/restart")
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("content-type", "application/json");
        if authenticated {
            request = request.header("authorization", format!("Bearer {}", self.token));
        }
        let response = self
            .app
            .clone()
            .oneshot(
                request
                    .body(Body::from(json!({"confirmed":confirmed}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }
    async fn request(&self, id: &str) -> Submission {
        let d = self.c.description("fixture").await.unwrap();
        serde_json::from_value(json!({"id":id,"tool":"fixture","action":"mark_band","revision":d.revision,"params":{"label":"Restart test"},"images":{"source":[self.image]}})).unwrap()
    }
    async fn idle(&self) {
        tokio::time::timeout(Duration::from_secs(8), async {
            while self.c.snapshot().await.unwrap()["tools"][0]["busy"] == true {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("tool did not become idle");
    }
    async fn close(&self, force: bool) {
        let (quit, mut stopped) = tokio::sync::watch::channel(false);
        self.c.begin_shutdown(true, quit.clone()).await.unwrap();
        if force {
            tokio::time::timeout(Duration::from_secs(8), async {
                while self.c.shutdown_status().await.phase != "waiting" {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            self.c.force_shutdown(quit).await.unwrap();
        }
        tokio::time::timeout(Duration::from_secs(8), stopped.wait_for(|done| *done))
            .await
            .unwrap()
            .unwrap();
    }
}
#[tokio::test]
async fn restart_requires_confirmation_and_invalidates_old_drafts_but_preserves_evidence() {
    let h = Host::new("normal").await;
    let before = h.c.description("fixture").await.unwrap();
    assert_eq!(h.restart(false, true).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(h.restart(true, false).await.0, StatusCode::CONFLICT);
    assert_eq!(
        h.c.description("fixture").await.unwrap().tool_session,
        before.tool_session
    );
    let mut stale = h.request("saved").await;
    h.c.submit(stale.clone()).await.unwrap();
    h.idle().await;
    let saved = h.c.attempts.get("saved").await.unwrap();
    let (status, after) = h.restart(true, true).await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_ne!(after["tool_session"]["id"], before.tool_session["id"]);
    assert_ne!(after["revision"], before.revision);
    stale.id = "stale".into();
    assert!(h.c.submit(stale).await.is_err());
    assert_eq!(
        serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap(),
        serde_json::to_value(saved).unwrap()
    );
    h.c.submit(h.request("fresh").await).await.unwrap();
    h.idle().await;
    assert_eq!(
        h.c.attempts.get("fresh").await.unwrap().execution,
        "completed"
    );
    h.close(false).await;
}
#[tokio::test]
async fn submit_and_restart_have_one_winner_and_active_work_is_never_replaced() {
    let h = Host::new("held").await;
    let request = h.request("race").await;
    let (submitted, restarted) = tokio::join!(h.c.submit(request), h.restart(true, true));
    assert_ne!(submitted.is_ok(), restarted.0 == StatusCode::OK);
    if submitted.is_err() {
        h.c.submit(h.request("race").await).await.unwrap();
    }
    let before = h.c.description("fixture").await.unwrap();
    assert_eq!(h.restart(true, true).await.0, StatusCode::CONFLICT);
    assert_eq!(
        h.c.description("fixture").await.unwrap().tool_session,
        before.tool_session
    );
    std::fs::write(h.root.path().join("tool/release-run"), "").unwrap();
    h.idle().await;
    h.close(false).await;
}
#[tokio::test]
async fn failed_replacement_is_disconnected_and_a_later_explicit_restart_recovers() {
    let h = Host::new("normal").await;
    std::fs::write(h.root.path().join("tool/scenario"), "wrong-protocol").unwrap();
    assert_ne!(h.restart(true, true).await.0, StatusCode::OK);
    let state = h.c.snapshot().await.unwrap();
    assert!(state["tools"][0]["declaration"].is_null());
    assert!(state["tools"][0]["error"].is_string());
    assert!(h.c.session("fixture").await.is_err());
    std::fs::write(h.root.path().join("tool/scenario"), "normal").unwrap();
    assert_eq!(h.restart(true, true).await.0, StatusCode::OK);
    h.close(false).await;
}
#[tokio::test]
async fn a_stuck_probe_prevents_replacement_without_forcing_it() {
    let h = Host::new("ignore-eof").await;
    let (status, _) = h.restart(true, true).await;
    let disconnected = h.c.description("fixture").await.is_err();
    h.close(true).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(disconnected);
}
#[tokio::test]
async fn a_confirmed_exited_child_can_restart_without_rewriting_unknown_history() {
    let h = Host::new("exit-during-call").await;
    h.c.submit(h.request("unknown").await).await.unwrap();
    h.idle().await;
    let saved = h.c.attempts.get("unknown").await.unwrap();
    assert!(saved.result.is_none());
    std::fs::write(h.root.path().join("tool/scenario"), "normal").unwrap();
    assert_eq!(h.restart(true, true).await.0, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(h.c.attempts.get("unknown").await.unwrap()).unwrap(),
        serde_json::to_value(saved).unwrap()
    );
    h.close(false).await;
}

#[tokio::test]
async fn abandoning_the_restart_request_does_not_abandon_its_owned_process() {
    let h = Host::new("normal").await;
    let before = h.c.description("fixture").await.unwrap();
    std::fs::write(h.root.path().join("tool/scenario"), "initialize-held").unwrap();
    let c = h.c.clone();
    let request = tokio::spawn(async move { c.restart("fixture", true).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !h.root.path().join("tool/initializing").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    request.abort();
    assert!(request.await.err().unwrap().is_cancelled());
    std::fs::write(h.root.path().join("tool/release-initialize"), "").unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let state = h.c.snapshot().await.unwrap();
            let tool = &state["tools"][0];
            if tool["busy"] == false
                && tool["declaration"]["tool_session"]["id"].is_string()
                && tool["declaration"]["tool_session"]["id"] != before.tool_session["id"]
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    h.close(false).await;
    outcome.expect("replacement was abandoned with its caller");
}
