use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Duration,
};
use tower::ServiceExt;
#[path = "render_controls/documents.rs"]
mod documents;
#[path = "render_controls/frames.rs"]
mod frames;
use wafflery::{
    host::{api, auth::Auth},
    library::{assets::AssetStore, attempts::Submission},
    tools::{coordinator::Coordinator, registration::Descriptor},
};
struct Host {
    _root: tempfile::TempDir,
    c: Arc<Coordinator>,
    app: Router,
    token: String,
    run: String,
}
impl Host {
    async fn new() -> Self {
        static BIN: OnceLock<PathBuf> = OnceLock::new();
        let bin = BIN.get_or_init(|| {
            assert!(
                std::process::Command::new(env!("CARGO"))
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
        let assets = AssetStore::open(&root.path().join("Library")).unwrap();
        let input = root.path().join("source.png");
        std::fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let image = assets.import_path(input).await.unwrap();
        let c = Arc::new(
            Coordinator::open(
                assets.clone(),
                vec![Descriptor {
                    id: "sample".into(),
                    executable: bin.clone(),
                    args: vec![tool.display().to_string(), "renders".into()],
                    config: None,
                    cwd: tool.clone(),
                    allow_dirs: vec![tool.clone()],
                    evidence_dirs: vec![tool],
                }],
            )
            .await
            .unwrap(),
        );
        let declaration = c.description("sample").await.unwrap();
        let request:Submission=serde_json::from_value(json!({"id":"saved","tool":"sample","action":"mark_band","revision":declaration.revision,"params":{"label":"saved render input"},"images":{"source":[image.id]}})).unwrap();
        c.submit(request).await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if c.attempts.get("saved").await.unwrap().preservation["contract"] == "complete"
                    && c.snapshot().await.unwrap()["tools"][0]["busy"] == false
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let run = c.attempts.get("saved").await.unwrap().result.unwrap()["run"]["dir"]
            .as_str()
            .unwrap()
            .to_owned();
        let auth = Auth::new().unwrap();
        let token = auth
            .exchange(&auth.issue().unwrap())
            .unwrap()
            .unwrap()
            .token;
        let (quit, _) = tokio::sync::watch::channel(false);
        let app =
            api::router_with_tools("http://127.0.0.1:4811", auth, quit, assets, Some(c.clone()));
        Self {
            _root: root,
            c,
            app,
            token,
            run,
        }
    }
    async fn call(
        &self,
        method: &str,
        path: &str,
        body: Value,
        token: &str,
        origin: &str,
    ) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("host", "127.0.0.1:4811")
                    .header("origin", origin)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn request(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(method, path, body, &self.token, "http://127.0.0.1:4811")
            .await
    }
    async fn state(&self, id: &str, phase: &str) -> Value {
        tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                let (status, value) = self
                    .request("GET", &format!("/api/views/{id}"), json!({}))
                    .await;
                assert_eq!(status, StatusCode::OK);
                if value["phase"] == phase {
                    break value;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn finish(&self) {
        let (q, mut done) = tokio::sync::watch::channel(false);
        self.c.begin_shutdown(true, q).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), done.wait_for(|v| *v))
            .await
            .unwrap()
            .unwrap();
        self.c.assets.database.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn report_controls_are_private_strict_idempotent_and_bound_to_saved_evidence() {
    let h = Host::new().await;
    let path = "/api/attempts/saved/views";
    assert_eq!(
        h.call(
            "POST",
            path,
            json!({"name":"hold"}),
            "wrong",
            "http://127.0.0.1:4811"
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "POST",
            path,
            json!({"name":"hold"}),
            &h.token,
            "https://other.invalid"
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    for body in [
        json!({"name":"hold","command":["/bin/sh"]}),
        json!({"name":"hold","frame_ancestor":"https://other.invalid"}),
        json!({"name":"unknown"}),
    ] {
        assert_eq!(
            h.request("POST", path, body).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let before = serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap();
    let (one, two) = tokio::join!(
        h.request("POST", path, json!({"name":"hold"})),
        h.request("POST", path, json!({"name":"hold"}))
    );
    assert_eq!(one.0, StatusCode::OK);
    assert_eq!(two.0, StatusCode::OK);
    assert_eq!(one.1["id"], two.1["id"]);
    let id = one.1["id"].as_str().unwrap();
    assert_eq!(one.1["attempt"], "saved");
    assert!(one.1.get("stdout").is_none());
    assert_eq!(
        h.request("POST", path, json!({"name":"echo"})).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.request("POST", &format!("/api/views/{id}/close"), json!({}))
            .await
            .0,
        StatusCode::OK
    );
    h.state(id, "exited").await;
    let (status, next) = h.request("POST", path, json!({"name":"early-error"})).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(next["id"], id);
    let state = h.state(next["id"].as_str().unwrap(), "exited").await;
    assert_eq!(state["exit_code"], 17);
    assert!(
        state["diagnostics"]
            .as_str()
            .unwrap()
            .contains("Synthetic render failure")
    );
    assert_eq!(
        serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap(),
        before
    );
    std::fs::rename(&h.run, format!("{}.moved", h.run)).unwrap();
    assert!(
        !h.request("POST", path, json!({"name":"hold"}))
            .await
            .0
            .is_success()
    );
    h.finish().await;
}
#[tokio::test]
async fn shutdown_keeps_a_stuck_view_owned_until_confirmed_force_and_blocks_reopen() {
    let h = Host::new().await;
    let (status, opened) = h
        .request("POST", "/api/attempts/saved/views", json!({"name":"stuck"}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let id = opened["id"].as_str().unwrap();
    assert_eq!(
        h.request(
            "POST",
            &format!("/api/views/{id}/force-close"),
            json!({"confirmed":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (q, mut done) = tokio::sync::watch::channel(false);
    h.c.begin_shutdown(true, q.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(6), async {
        while h.c.shutdown_status().await.phase != "waiting" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!*done.borrow());
    h.state(id, "needs_force").await;
    assert_eq!(
        h.request("POST", "/api/attempts/saved/views", json!({"name":"hold"}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.request(
            "POST",
            &format!("/api/views/{id}/force-close"),
            json!({"confirmed":false})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    h.c.force_shutdown(q).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), done.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
    h.state(id, "exited").await;
    assert_eq!(
        h.c.attempts.get("saved").await.unwrap().execution,
        "completed"
    );
    h.c.assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn report_capacity_is_bounded_and_confirmed_exit_releases_a_slot() {
    let h = Host::new().await;
    let original = h.c.attempts.get("saved").await.unwrap();
    let mut ids = Vec::new();
    for index in 0..17 {
        let attempt = if index == 0 {
            "saved".to_owned()
        } else {
            let mut request = original.request.clone();
            request.id = format!("saved-{index}");
            h.c.submit(request.clone()).await.unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if h.c.attempts.get(&request.id).await.unwrap().preservation["contract"]
                        == "complete"
                        && h.c.snapshot().await.unwrap()["tools"][0]["busy"] == false
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            request.id
        };
        let path = format!("/api/attempts/{attempt}/views");
        let (status, view) = h.request("POST", &path, json!({"name":"hold"})).await;
        if index < 16 {
            assert_eq!(status, StatusCode::OK);
            ids.push(view["id"].as_str().unwrap().to_owned());
        } else {
            assert_eq!(status, StatusCode::CONFLICT);
            h.request("POST", &format!("/api/views/{}/close", ids[0]), json!({}))
                .await;
            h.state(&ids[0], "exited").await;
            assert_eq!(
                h.request("POST", &path, json!({"name":"hold"})).await.0,
                StatusCode::OK
            );
        }
    }
    h.finish().await;
}

#[tokio::test]
async fn server_readiness_is_supervised_without_polling_and_extra_output_closes_the_view() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Host::new().await;
    let before = serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap();
    let tool = h._root.path().join("tool");
    let (status, view) = h
        .request(
            "POST",
            "/api/attempts/saved/views",
            json!({"name":"server-delayed"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let id = view["id"].as_str().unwrap();
    assert_eq!(view["phase"], "starting");
    std::fs::write(tool.join("render-release"), "ready").unwrap();
    let ready = h.state(id, "ready").await;
    assert!(ready.get("url").is_none());
    assert!(!ready.to_string().contains("http://"));
    let address =
        url::Url::parse(&std::fs::read_to_string(tool.join("render-address")).unwrap()).unwrap();
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", address.port().unwrap()))
        .await
        .unwrap();
    stream
        .write_all(b"GET /report HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), stream.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(response.contains("frame-ancestors http://127.0.0.1:4811"));
    assert!(response.contains("Saved report"));
    h.request("POST", &format!("/api/views/{id}/close"), json!({}))
        .await;
    let closed = h.state(id, "exited").await;
    assert_eq!(closed["exit_code"], 0);
    assert!(closed["error"].is_null());
    let (_, view) = h
        .request(
            "POST",
            "/api/attempts/saved/views",
            json!({"name":"server-extra"}),
        )
        .await;
    let id = view["id"].as_str().unwrap();
    h.state(id, "ready").await;
    std::fs::write(tool.join("render-extra"), "extra").unwrap();
    // No GET drives cleanup: observe the independent process releasing its listener first.
    let address =
        url::Url::parse(&std::fs::read_to_string(tool.join("render-address")).unwrap()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while tokio::net::TcpStream::connect(("127.0.0.1", address.port().unwrap()))
            .await
            .is_ok()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let failed = h.state(id, "exited").await;
    assert!(failed["error"].as_str().unwrap().contains("extra stdout"));
    assert_eq!(
        serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap(),
        before
    );
    h.finish().await;
}

#[tokio::test]
async fn server_startup_timeout_bad_url_and_early_exit_preserve_the_run() {
    let h = Host::new().await;
    let before = serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap();
    for (name, code, error) in [
        ("server-timeout", 0, "deadline"),
        ("server-bad", 0, "loopback"),
        ("server-exit", 17, "Report"),
    ] {
        let (status, view) = h
            .request("POST", "/api/attempts/saved/views", json!({"name":name}))
            .await;
        assert_eq!(status, StatusCode::OK);
        let failed = h.state(view["id"].as_str().unwrap(), "exited").await;
        assert_eq!(failed["exit_code"], code);
        assert!(
            failed["error"].as_str().unwrap().contains(error),
            "{failed}"
        );
        assert_eq!(
            serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap(),
            before
        );
    }
    h.finish().await;
}

#[tokio::test]
async fn report_activity_is_private_strict_and_cannot_revive_closed_or_replaced_views() {
    let h = Host::new().await;
    let (_, view) = h
        .request("POST", "/api/attempts/saved/views", json!({"name":"hold"}))
        .await;
    let id = view["id"].as_str().unwrap();
    let path = format!("/api/views/{id}/activity");
    assert_eq!(
        h.call("POST", &path, json!({}), "wrong", "http://127.0.0.1:4811")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.request("POST", &path, json!({"ttl":86400})).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, active) = h.request("POST", &path, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(active["idle_expired"], false);
    h.request("POST", &format!("/api/views/{id}/close"), json!({}))
        .await;
    assert_eq!(
        h.request("POST", &path, json!({})).await.0,
        StatusCode::CONFLICT
    );
    let (_, replacement) = h
        .request("POST", "/api/attempts/saved/views", json!({"name":"hold"}))
        .await;
    assert_ne!(replacement["id"], view["id"]);
    assert_eq!(
        h.request("POST", &path, json!({})).await.0,
        StatusCode::NOT_FOUND
    );
    h.finish().await;
}

#[tokio::test]
async fn current_report_lookup_is_private_and_never_starts_or_replaces_a_view() {
    let h = Host::new().await;
    let path = "/api/attempts/saved/views";
    assert_eq!(
        h.call("GET", path, json!({}), "wrong", "http://127.0.0.1:4811")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, empty) = h.request("GET", path, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty, json!({"view":null}));
    let opened = h.request("POST", path, json!({"name":"document"})).await.1;
    let id = opened["id"].as_str().unwrap();
    h.state(id, "ready").await;
    let (_, current) = h.request("GET", path, json!({})).await;
    assert_eq!(current["view"]["id"], id);
    assert_eq!(current["view"]["phase"], "ready");
    h.request("POST", &format!("/api/views/{id}/close"), json!({}))
        .await;
    let (_, current) = h.request("GET", path, json!({})).await;
    assert_eq!(current["view"]["id"], id);
    assert_eq!(current["view"]["phase"], "exited");
    h.finish().await;
}
