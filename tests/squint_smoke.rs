//! Explicit local testkit executable only; no credentials or live provider calls.
#[path = "support/real_peers.rs"]
mod peer;
use serde_json::json;
use std::sync::Arc;
use wafflery::{
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};

#[tokio::test]
#[ignore = "requires WAFFLERY_SQUINT_BIN pointing to a contract 0.5.0 testkit binary"]
async fn real_squint_scripted_check_preserves_zero_spend_evidence() {
    let binary = peer::binary("WAFFLERY_SQUINT_BIN");
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir(&tool).unwrap();
    let config = tool.join("config.toml");
    let script = tool.join("held.json");
    std::fs::write(&config, include_str!("../testkit/squint/call-config.toml")).unwrap();
    std::fs::write(
        &script,
        include_str!("../testkit/squint/references-held.json"),
    )
    .unwrap();
    let input = dir.path().join("original.png");
    let bytes = include_bytes!("../ui/tests/fixtures/sample.png");
    std::fs::write(&input, bytes).unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let image = assets.import_path(input.clone()).await.unwrap();
    let reference_path = dir.path().join("reference.png");
    std::fs::write(
        &reference_path,
        include_bytes!("../ui/tests/fixtures/panorama.png"),
    )
    .unwrap();
    let reference = assets.import_path(reference_path).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            assets,
            vec![Descriptor {
                config: Some(config),
                id: "squint".into(),
                executable: binary,
                args: vec![
                    "serve".into(),
                    "--provider-script".into(),
                    script.display().to_string(),
                    "--out".into(),
                    tool.join("runs").display().to_string(),
                ],
                cwd: tool.clone(),
                allow_dirs: vec![],
                evidence_dirs: vec![tool.clone()],
            }],
        )
        .await
        .unwrap(),
    );
    let (saved, bundle_bytes) = peer::run(&c, async {
        let description = c.description("squint").await.unwrap();
        assert_eq!(description.document["contract"], "0.5.0");
        assert!(
            description.document["actions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["kind"] == "task")
        );
        for action in description.document["actions"].as_array().unwrap() {
            assert_eq!(action["cost"]["may_spend"], false, "{action}");
            assert_eq!(action["readiness"]["state"], "ready", "{action}");
        }
        assert!(
            !description.document["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["name"] == "estimate")
        );
        assert!(description.schemas.contains_key("estimate"));
        assert!(
            c.session("squint").await.unwrap()["allowed_dirs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p.as_str().unwrap().ends_with("/execution"))
        );
        let id = "squint-probe";
        let request = serde_json::from_value(json!({
            "id":id, "tool":"squint", "action":"reference_consistency",
            "revision":description.revision, "params":{},
            "images":{"candidate":[image.id],"references":[reference.id,image.id,reference.id]}
        }))
        .unwrap();
        c.submit(request).await.unwrap();
        let saved = peer::settled(&c, id).await;
        peer::reports(&c, &saved).await;
        let bundle = wafflery::tools::bundles::build(&c.assets, &saved, &Default::default())
            .await
            .unwrap();
        let bytes = std::fs::read(bundle.file.path()).unwrap();
        (saved, bytes)
    })
    .await;
    assert_eq!(
        saved.execution,
        "completed",
        "{:?}; {:?}",
        saved.notices,
        saved.result.as_ref().map(|v| &v["error"])
    );
    assert_eq!(
        saved.preservation["contract"], "complete",
        "{:?}",
        saved.notices
    );
    assert!(saved.wire.is_some());
    let result = saved.result.unwrap();
    assert_eq!(result["spend"]["usd"], 0.0);
    assert_eq!(result["body"]["verdict"], "pass");
    let wire = &saved.wire.as_ref().unwrap()["params"]["arguments"]["references"];
    assert_eq!(wire.as_array().unwrap().len(), 3);
    assert_eq!(wire[0]["expect"]["file_sha256"], reference.sha256);
    assert_eq!(wire[1]["expect"]["file_sha256"], image.sha256);
    assert_eq!(wire[2]["expect"], wire[0]["expect"]);
    assert_ne!(wire[0]["path"], wire[2]["path"]);
    for (index, asset) in [reference.id.clone(), image.id.clone(), reference.id]
        .iter()
        .enumerate()
    {
        assert_eq!(saved.inputs["references"][index]["asset"]["id"], *asset);
    }
    assert!(saved.outputs.is_empty());
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    // Direct calls enforce the same explicit path rule; the host's call above used staging grants.
    let mut arguments = result["invocation"]["arguments"].clone();
    arguments["candidate"]["path"] = json!(input);
    for (index, name) in ["reference.png", "original.png", "reference.png"]
        .iter()
        .enumerate()
    {
        arguments["references"][index]["path"] = json!(dir.path().join(name));
    }
    let cli = dir.path().join("cli");
    std::fs::create_dir(&cli).unwrap();
    std::fs::write(
        cli.join("config.toml"),
        include_str!("../testkit/squint/call-config.toml"),
    )
    .unwrap();
    std::fs::write(
        cli.join("script.json"),
        include_str!("../testkit/squint/references-held.json"),
    )
    .unwrap();
    let args = cli.join("arguments.json");
    std::fs::write(&args, serde_json::to_vec(&arguments).unwrap()).unwrap();
    let executable = peer::binary("WAFFLERY_SQUINT_BIN");
    let command = || {
        let mut command = tokio::process::Command::new(&executable);
        command
            .arg("call")
            .arg("--config")
            .arg(cli.join("config.toml"))
            .arg("--provider-script")
            .arg(cli.join("script.json"))
            .arg("--out")
            .arg(cli.join("runs"))
            .current_dir(&cli);
        command
    };
    let denied = peer::output(
        command()
            .arg("reference_consistency")
            .arg("--arguments")
            .arg(&args),
    )
    .await;
    assert_eq!(denied.status.code(), Some(64));
    let denied: serde_json::Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(denied["status"], "invalid");
    assert_eq!(denied["error"]["code"], "input");
    assert!(
        denied["error"]["reason"]
            .as_str()
            .unwrap()
            .contains("outside the directories")
    );
    let allowed = peer::output(
        command()
            .arg("reference_consistency")
            .arg("--arguments")
            .arg(&args)
            .arg("--allow-dir")
            .arg(dir.path()),
    )
    .await;
    assert!(
        allowed.status.success(),
        "{}",
        String::from_utf8_lossy(&allowed.stderr)
    );
    std::fs::remove_dir_all(&tool).unwrap();
    std::fs::remove_dir_all(dir.path().join("Library")).unwrap();
    std::fs::remove_file(&input).unwrap();
    std::fs::remove_file(dir.path().join("reference.png")).unwrap();
    std::fs::remove_file(&args).unwrap();
    let archive = cli.join("check.tar");
    std::fs::write(&archive, bundle_bytes).unwrap();
    let replay = peer::output(
        command()
            .arg("--bundle")
            .arg(&archive)
            .arg("--allow-dir")
            .arg(&cli),
    )
    .await;
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let replay: serde_json::Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(replay["status"], "completed");
    assert_eq!(replay["invocation"], result["invocation"]);
    assert_eq!(replay["images"]["in"], result["images"]["in"]);
    assert_eq!(replay["spend"]["usd"], 0.0);
}

#[tokio::test]
#[ignore = "requires the approved Squint binary, isolated XDG directories and no ANTHROPIC_API_KEY"]
async fn real_squint_discovers_setup_and_prices_without_allocating_a_run() {
    assert!(
        std::env::var_os("ANTHROPIC_API_KEY").is_none(),
        "Run this no-credential probe with ANTHROPIC_API_KEY unset"
    );
    let binary = peer::binary("WAFFLERY_SQUINT_BIN");
    for selected in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join("tool");
        std::fs::create_dir(&tool).unwrap();
        let config = if selected {
            dir.path().join("selected config.toml")
        } else {
            tool.join("squint.toml")
        };
        let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
        let input = dir.path().join("image.png");
        std::fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let image = assets.import_path(input).await.unwrap();
        let c = Arc::new(
            Coordinator::open(
                assets,
                vec![Descriptor {
                    id: "squint".into(),
                    executable: binary.clone(),
                    config: selected.then(|| config.clone()),
                    args: vec![
                        "serve".into(),
                        "--out".into(),
                        tool.join("runs").display().to_string(),
                    ],
                    cwd: tool.clone(),
                    allow_dirs: vec![],
                    evidence_dirs: vec![tool.clone()],
                }],
            )
            .await
            .unwrap(),
        );
        peer::run(&c, async {
            let missing = c.description("squint").await.unwrap();
            let action = |description: &wafflery::tools::client::Declaration| {
                description.document["actions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|a| a["name"] == "source_fidelity")
                    .unwrap()
                    .clone()
            };
            assert_eq!(action(&missing)["readiness"]["state"], "needs_setup");
            assert!(
                action(&missing)["readiness"]["requirements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["kind"] == "config" && r["state"] == "missing")
            );
            std::fs::write(
                &config,
                include_str!("../testkit/squint/estimate-config.toml"),
            )
            .unwrap();
            assert_eq!(
                c.refresh("squint").await.unwrap().tool_session["id"],
                missing.tool_session["id"]
            );
            assert_eq!(
                action(&c.description("squint").await.unwrap())["readiness"],
                action(&missing)["readiness"]
            );
            let configured = c.restart("squint", true).await.unwrap();
            assert_ne!(configured.tool_session["id"], missing.tool_session["id"]);
            let current = action(&configured);
            assert_eq!(current["cost"]["may_spend"], true);
            assert_eq!(current["readiness"]["state"], "needs_setup");
            assert!(
                current["readiness"]["requirements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["kind"] == "credential"
                        && r["name"] == "ANTHROPIC_API_KEY"
                        && r["state"] == "missing")
            );
            let request =
                serde_json::from_value::<wafflery::library::attempts::Submission>(json!({
                    "id":"estimate-control", "tool":"squint", "action":"source_fidelity",
                    "revision":configured.revision, "params":{"max_spend_usd":2.0},
                    "images":{"candidate":[image.id],"source":[image.id]}
                }))
                .unwrap();
            c.prepare(request.clone()).await.unwrap();
            let quote = c.estimate_prepared(&request.id).await.unwrap();
            assert_eq!(quote.estimate["allowed"], false);
            assert_eq!(quote.estimate["refused"], "readiness");
            assert!(quote.estimate["worst_case_usd"].as_f64().unwrap() > 0.0);
            assert_eq!(quote.estimate["effective"]["model"], "claude-haiku-5-5");
            assert!(
                quote.estimate["effective"]["config_identity"]
                    .as_str()
                    .unwrap()
                    .starts_with("sha256:")
            );
            assert!(c.submit_prepared(request.clone()).await.is_err());
            assert_eq!(c.snapshot().await.unwrap()["attempts"], json!([]));
            let runs = tool.join("runs");
            assert!(
                !runs.exists() || std::fs::read_dir(runs).unwrap().next().is_none(),
                "An estimate must not create a tool run"
            );
            assert_eq!(c.session("squint").await.unwrap()["spent_usd"], 0.0);
            c.restart("squint", true).await.unwrap();
            assert!(c.prepared_estimate(&request.id).await.is_err());
            assert_eq!(c.snapshot().await.unwrap()["attempts"], json!([]));
        })
        .await;
    }
}
