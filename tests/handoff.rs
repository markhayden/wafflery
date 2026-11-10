use serde_json::{Value, json};
use wafflery::library::{
    assets::AssetStore,
    attempts::{Attempt, AttemptStore, Submission},
};
use wafflery::tools::handoff;

async fn fixture() -> (tempfile::TempDir, AssetStore, Attempt) {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Local library ' $() ;")).unwrap();
    let path = root.path().join("input.png");
    std::fs::write(&path, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let image = assets.import_path(path).await.unwrap();
    let request:Submission=serde_json::from_value(json!({"id":"saved","tool":"peer","action":"shape","revision":"saved","params":{"amount":0.2},"images":{"source":[image.id]}})).unwrap();
    let declaration = json!({"document":{"contract":"0.6.0","tool":{"name":"study","version":"1.0.0"},"actions":[{"name":"shape","kind":"task","images":[{"slot":"source","required":true}]}]}});
    let mut attempt=AttemptStore::new(assets.database.clone()).create(request,json!({}),declaration,json!({"source":[{"asset":image,"image":{"file_sha256":image.sha256,"width":image.width,"height":image.height,"projection":"flat","topology":"bounded"},"role":"source","argument_index":0}]})).await.unwrap();
    attempt.result = Some(
        json!({"contract":"0.6.0","status":"completed","action":"shape","invocation":{"action":"shape","arguments":{"source":{"expect":{"file_sha256":image.sha256,"width":image.width,"height":image.height,"projection":"flat","topology":"bounded"}},"amount":0.75,"max_spend_usd":0.3},"effective":{"model":"saved-model","config_identity":format!("sha256:{}","a".repeat(64))},"requirements":["PROVIDER_KEY","settings.toml"]}}),
    );
    attempt.execution = "completed".into();
    attempt.preservation["contract"] = json!("complete");
    attempt.observations = json!({"host_launch":{"executable":"/usr/bin/true","cwd":root.path(),"config":root.path().join("settings ' $(no).toml"),"unresolved_arguments":false}});
    (root, assets, attempt)
}
#[tokio::test]
async fn handoff_materializes_only_saved_inputs_and_resolved_values_with_quoted_argv() {
    let (root, assets, attempt) = fixture().await;
    let handoff = handoff::build(&assets, &attempt).await.unwrap();
    assert_eq!(handoff.state, "local", "{:?}", handoff.blockers);
    assert!(handoff.blockers.is_empty());
    let arguments = handoff.arguments.as_ref().unwrap();
    assert_eq!(arguments["amount"], 0.75);
    assert_eq!(arguments["max_spend_usd"], 0.3);
    assert!(
        arguments["source"]["path"]
            .as_str()
            .unwrap()
            .starts_with(assets.root().to_str().unwrap())
    );
    assert_eq!(
        arguments["source"]["expect"],
        attempt.result.as_ref().unwrap()["invocation"]["arguments"]["source"]["expect"]
    );
    assert!(
        handoff
            .argv
            .as_ref()
            .unwrap()
            .contains(&"--allow-dir".into())
    );
    assert!(handoff.command.as_ref().unwrap().contains("'\\''"));
    assert!(
        handoff.command.as_ref().unwrap().contains("$(no)"),
        "literal shell syntax stays inside quoted data"
    );
    assert_eq!(handoff.requirements, vec!["PROVIDER_KEY", "settings.toml"]);
    assert!(
        !root.path().join("settings ' $(no).toml").exists(),
        "the host never reads or creates tool config"
    );
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn unresolved_launch_arguments_old_invocations_and_missing_owned_bytes_withhold_commands() {
    let (_root, assets, mut attempt) = fixture().await;
    attempt.observations["host_launch"]["unresolved_arguments"] = json!(true);
    let blocked = handoff::build(&assets, &attempt).await.unwrap();
    assert_eq!(blocked.state, "unavailable");
    assert!(blocked.command.is_none());
    attempt.observations = json!({});
    assert!(
        handoff::build(&assets, &attempt)
            .await
            .unwrap()
            .command
            .is_none()
    );
    attempt.result.as_mut().unwrap()["invocation"] = Value::Null;
    attempt.result.as_mut().unwrap()["status"] = json!("invalid");
    assert!(
        handoff::build(&assets, &attempt)
            .await
            .unwrap()
            .arguments
            .is_none()
    );
    assets.database.shutdown().await.unwrap();
}

#[test]
fn launch_snapshots_keep_supported_paths_but_never_unknown_argument_values() {
    use wafflery::tools::registration::Descriptor;
    let root = tempfile::tempdir().unwrap();
    let mut descriptor = Descriptor {
        id: "peer".into(),
        executable: "/usr/bin/true".into(),
        args: vec!["serve".into(), "--config".into(), "settings.toml".into()],
        config: None,
        cwd: root.path().into(),
        allow_dirs: vec![],
        evidence_dirs: vec![],
    };
    let launch = handoff::launch_context(&descriptor);
    assert!(!launch.unresolved_arguments);
    assert_eq!(
        launch.config.as_deref(),
        root.path().join("settings.toml").to_str()
    );
    descriptor.args = vec![
        "serve".into(),
        "--private-value".into(),
        "--config=not-a-config-secret".into(),
    ];
    let launch = handoff::launch_context(&descriptor);
    assert!(launch.unresolved_arguments);
    assert!(
        !serde_json::to_string(&launch)
            .unwrap()
            .contains("not-a-config-secret")
    );
}
#[tokio::test]
async fn missing_owned_inputs_block_materialized_arguments_instead_of_using_outputs_or_preview_bytes()
 {
    let (_root, assets, attempt) = fixture().await;
    let original = handoff::build(&assets, &attempt).await.unwrap();
    let path = original.arguments.unwrap()["source"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::remove_file(path).unwrap();
    let unavailable = handoff::build(&assets, &attempt).await.unwrap();
    assert_eq!(unavailable.state, "unavailable");
    assert!(unavailable.command.is_none());
    assert!(
        unavailable
            .blockers
            .iter()
            .any(|reason| reason.contains("missing or corrupt"))
    );
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn mixed_image_references_and_owned_list_items_keep_their_global_occurrence_indices() {
    let (_root, assets, mut attempt) = fixture().await;
    let saved = attempt.inputs["source"][0].clone();
    let expect = attempt.result.as_ref().unwrap()["invocation"]["arguments"]["source"].clone();
    let hash = expect["expect"]["file_sha256"].as_str().unwrap();
    let reference = json!({"type":"image","schema_version":1,"tool":"study","identity":format!("sha256:{hash}"),"resource":format!("study://artifact/sha256:{hash}"),"frame":null});
    attempt.declaration["document"]["actions"][0] = json!({"name":"shape","kind":"operation","ports":{"in":[{"name":"pictures","type":"image","many":true,"required":true}]}});
    let mut one = saved.clone();
    one["argument_index"] = json!(1);
    let mut two = saved;
    two["argument_index"] = json!(2);
    attempt.inputs = json!({"pictures":[one,two]});
    attempt.result.as_mut().unwrap()["invocation"]["arguments"] =
        json!({"pictures":[reference,expect,expect]});
    let handoff = handoff::build(&assets, &attempt).await.unwrap();
    assert_eq!(handoff.state, "local", "{:?}", handoff.blockers);
    let items = handoff.arguments.unwrap()["pictures"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(items[0], reference);
    assert_eq!(items[1]["path"], items[2]["path"]);
    assert!(items[0].get("path").is_none());
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn authenticated_argument_download_uses_saved_evidence_and_accepts_no_caller_paths() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    use wafflery::{
        host::{api, auth::Auth},
        tools::coordinator::Coordinator,
    };
    let (_root, assets, attempt) = fixture().await;
    AttemptStore::new(assets.database.clone())
        .update("saved", move |saved| {
            *saved = attempt;
            Ok(())
        })
        .await
        .unwrap();
    let c = Arc::new(Coordinator::open(assets.clone(), vec![]).await.unwrap());
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let origin = "http://127.0.0.1:45678";
    let app = api::router_with_tools(origin, auth, quit, assets.clone(), Some(c.clone()));
    for (method, path, body, authorized, status) in [
        (
            "GET",
            "/api/attempts/saved/handoff",
            "{}",
            false,
            StatusCode::UNAUTHORIZED,
        ),
        (
            "GET",
            "/api/attempts/saved/handoff",
            "{}",
            true,
            StatusCode::OK,
        ),
        (
            "POST",
            "/api/attempts/saved/handoff/arguments",
            r#"{"path":"/unrelated/private.json"}"#,
            true,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "POST",
            "/api/attempts/saved/handoff/arguments",
            "{}",
            true,
            StatusCode::OK,
        ),
    ] {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", origin)
            .header("content-type", "application/json");
        if authorized {
            request = request.header("authorization", format!("Bearer {}", session.token));
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if method == "POST" && status == StatusCode::OK {
            assert_eq!(response.headers()["content-type"], "application/json");
            let arguments: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                    .unwrap();
            assert_eq!(arguments["amount"], 0.75);
            assert_eq!(arguments["max_spend_usd"], 0.3);
            assert!(arguments["source"]["path"].is_string());
        }
    }
    assert_eq!(
        c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(c.close().await);
    assets.database.shutdown().await.unwrap();
}
