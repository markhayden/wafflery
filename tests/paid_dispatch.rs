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
async fn paid_dispatch_uses_the_owned_quote_and_records_exact_wire_and_evidence() {
    let (dir, c, request) = setup("paid normal").await;
    c.prepare(request.clone()).await.unwrap();
    let quote = c.estimate_prepared(&request.id).await.unwrap();
    assert_eq!(quote.estimate["allowed"], true);
    let (submitted, simultaneous) = tokio::join!(
        c.submit_prepared(request.clone()),
        c.submit_prepared(request.clone())
    );
    if let Err(error) = submitted {
        finish(&c).await;
        panic!("allowed owned preparation should dispatch: {error}");
    }
    assert_eq!(simultaneous.unwrap().id, request.id);
    let saved = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let attempt = c.attempts.get(&request.id).await.unwrap();
            if attempt.preservation["contract"] != "pending" {
                break attempt;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let duplicate = c.submit_prepared(request.clone()).await.unwrap();
    assert_eq!(c.submit(request.clone()).await.unwrap().id, duplicate.id);
    let archive = wafflery::library::export::archive(&c.assets, saved.clone(), true)
        .await
        .unwrap();
    let decoder = flate2::read::GzDecoder::new(archive.file.reopen().unwrap());
    let mut archive = tar::Archive::new(decoder);
    let mut exported = None;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.path().unwrap() == std::path::Path::new("evidence/attempt.json") {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
            exported = Some(serde_json::from_slice::<Value>(&bytes).unwrap());
        }
    }
    assert_eq!(
        exported.unwrap()["observations"]["estimate"],
        saved.observations["estimate"]
    );
    let mut retry = request.clone();
    retry.id = "retry".into();
    assert!(c.submit_prepared(retry.clone()).await.is_err());
    c.prepare(retry.clone()).await.unwrap();
    assert!(
        c.submit_prepared(retry).await.is_err(),
        "another paid attempt needs a fresh estimate"
    );
    let calls = std::fs::read_to_string(dir.path().join("runs/calls.ndjson")).unwrap();
    finish(&c).await;
    assert_eq!(saved.execution, "completed", "{saved:?}");
    assert_eq!(duplicate.id, saved.id);
    assert_eq!(calls.lines().count(), 1);
    let wire: Value = serde_json::from_str(calls.lines().next().unwrap()).unwrap();
    assert_eq!(wire["name"], "price_test");
    assert_eq!(wire["arguments"], saved.arguments);
    assert_eq!(
        wire["_meta"]["table-manners/estimate"],
        saved.observations["estimate"]["binding"]
    );
    assert_eq!(wire["_meta"]["table-manners/client_run_id"], request.id);
    assert_eq!(
        saved.wire.as_ref().unwrap()["params"]["_meta"]["table-manners/estimate"],
        wire["_meta"]["table-manners/estimate"]
    );
    assert_eq!(saved.observations["estimate"]["cap_usd"], 0.5);
    assert_eq!(saved.result.as_ref().unwrap()["spend"]["usd"], 0.08);
    assert_eq!(saved.observations["session_after"]["spent_usd"], 0.08);
}

#[tokio::test]
async fn paid_submission_refuses_missing_refused_changed_expired_or_old_session_quotes() {
    for scenario in [
        "missing",
        "readiness",
        "cap",
        "params",
        "file",
        "expiry",
        "restart",
        "declaration",
    ] {
        let (dir, c, mut request) = setup(if scenario == "readiness" {
            "readiness"
        } else {
            "normal"
        })
        .await;
        if scenario == "cap" {
            request.params["max_spend_usd"] = json!(0.01);
        }
        assert!(
            c.submit(request.clone()).await.is_err(),
            "ordinary submit cannot authorize paid work"
        );
        assert!(
            c.submit_prepared(request.clone()).await.is_err(),
            "unprepared paid call"
        );
        c.prepare(request.clone()).await.unwrap();
        if scenario != "missing" {
            c.estimate_prepared(&request.id).await.unwrap();
        }
        match scenario {
            "params" => request.params["max_spend_usd"] = json!(0.6),
            "file" => {
                let arguments: Value = serde_json::from_str(
                    &std::fs::read_to_string(dir.path().join("runs/estimate-started")).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    arguments["source"]["path"].as_str().unwrap(),
                    b"changed after pricing",
                )
                .unwrap();
            }
            "expiry" => {
                tokio::time::pause();
                tokio::time::advance(Duration::from_secs(301)).await;
                tokio::time::resume();
            }
            "restart" => {
                c.restart("fixture", true).await.unwrap();
            }
            "declaration" => {
                std::fs::write(
                    dir.path().join("runs/estimate-declaration-changed"),
                    b"changed",
                )
                .unwrap();
            }
            _ => (),
        }
        let result = c.submit_prepared(request).await;
        let history = c.attempts.history(None, 50, None, None).await.unwrap();
        finish(&c).await;
        assert!(result.is_err(), "{scenario}");
        assert!(history.is_empty(), "{scenario}");
        assert!(!dir.path().join("runs/calls.ndjson").exists(), "{scenario}");
    }
}

#[tokio::test]
async fn prepared_paid_api_keeps_auth_identity_and_binding_checks_on_the_server() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (dir, c, input) = setup("api").await;
    let auth = wafflery::host::auth::Auth::new().unwrap();
    let token = auth
        .exchange(&auth.issue().unwrap())
        .unwrap()
        .unwrap()
        .token;
    let (quit, _) = tokio::sync::watch::channel(false);
    let origin = "http://127.0.0.1:9999";
    let app = wafflery::host::api::router_with_tools(
        origin,
        auth,
        quit,
        c.assets.clone(),
        Some(c.clone()),
    );
    let request = |path: &str, credential: &str, site: &str, body: Value| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("host", "127.0.0.1:9999")
            .header("origin", site)
            .header("authorization", format!("Bearer {credential}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    c.prepare(input.clone()).await.unwrap();
    c.estimate_prepared(&input.id).await.unwrap();
    let body = serde_json::to_value(&input).unwrap();
    let endpoint = "/api/preparations/prepared/submit";
    for (credential, site, status) in [
        ("wrong", origin, StatusCode::UNAUTHORIZED),
        (&token[..], "https://other.invalid", StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(endpoint, credential, site, body.clone()))
                .await
                .unwrap()
                .status(),
            status
        );
    }
    let mut invented = body.clone();
    invented["binding"] = json!("invented");
    assert_eq!(
        app.clone()
            .oneshot(request(endpoint, &token, origin, invented))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.clone()
            .oneshot(request("/api/attempts", &token, origin, body.clone()))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "/api/preparations/other/submit",
                &token,
                origin,
                body.clone()
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    let response = app
        .oneshot(request(endpoint, &token, origin, body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if c.attempts.get(&input.id).await.unwrap().preservation["contract"] != "pending" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    finish(&c).await;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[path = "paid_dispatch/outcomes.rs"]
mod outcomes;
