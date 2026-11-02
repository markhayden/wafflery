//! Opt-in, no credentials, no config, no filler. Not part of the required foundation gate.
#[path = "support/real_peers.rs"]
mod peer;
use serde_json::json;
use std::sync::Arc;
use wafflery::{
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};
#[tokio::test]
#[ignore = "requires WAFFLERY_RASCAL_BIN pointing to an explicitly selected local binary"]
async fn real_rascal_offline_seam_uses_the_same_dispatch_and_evidence_guards() {
    let binary = peer::binary("WAFFLERY_RASCAL_BIN");
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir(&tool).unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = dir.path().join("owned-source.png");
    let bytes = include_bytes!("../ui/tests/fixtures/panorama.png");
    std::fs::write(&input, bytes).unwrap();
    let asset = assets.import_path(input.clone()).await.unwrap();
    let descriptor = Descriptor {
        config: None,
        id: "real-offline-smoke".into(),
        executable: binary.clone(),
        args: vec![
            "serve".into(),
            "--workspace".into(),
            tool.join("workspace").display().to_string(),
            "--runs".into(),
            tool.join("runs").display().to_string(),
        ],
        cwd: tool.clone(),
        allow_dirs: vec![],
        evidence_dirs: vec![tool],
    };
    let c = Arc::new(Coordinator::open(assets, vec![descriptor]).await.unwrap());
    let outcome = peer::run(&c, async {
        let description = c.description("real-offline-smoke").await.unwrap();
        assert_eq!(description.document["contract"], "0.4.0");
        let seam = description.document["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == "seam")
            .unwrap();
        assert_eq!(seam["readiness"]["state"], "ready");
        assert_eq!(seam["cost"]["may_spend"], false);
        eprintln!(
            "Selected {} — tool {} {}, table-manners {}",
            binary.display(),
            description.document["tool"]["name"],
            description.document["tool"]["version"],
            description.document["contract"]
        );
        let request = serde_json::from_value(json!({
            "id": "offline-seam", "tool": "real-offline-smoke", "action": "seam",
            "revision": description.revision,
            "params": {"projection": "equirectangular"},
            "images": {"image": [asset.id]},
            "frames": {"image": [{"projection": "equirectangular", "topology": "wrap"}]}
        }))
        .unwrap();
        c.submit(request).await.unwrap();
        let outcome = peer::settled(&c, "offline-seam").await;
        peer::reports(&c, &outcome).await;
        outcome
    })
    .await;
    assert_eq!(std::fs::read(input).unwrap(), bytes);
    assert_eq!(outcome.execution, "completed", "{:?}", outcome.notices);
    assert_eq!(outcome.result.as_ref().unwrap()["spend"]["usd"], 0.0);
    assert_eq!(
        outcome.preservation["contract"], "complete",
        "{:?}",
        outcome.notices
    );
    assert!(
        !outcome.outputs.is_empty(),
        "Smoke must preserve a real output image"
    );
}

#[tokio::test]
#[ignore = "requires the approved Rascal testkit binary and isolated XDG directories"]
async fn real_rascal_binds_caps_and_restarts_with_a_new_config_and_ledger() {
    let binary = peer::binary("WAFFLERY_RASCAL_BIN");
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir(&tool).unwrap();
    let config = tool.join("selected config.toml");
    let configuration = |cost| {
        format!(
            "config_version = 1\n[limits]\ncall_cap_usd = 0.1\nsession_budget_usd = 0.05\n[fillers.local]\ncommand = [{:?}, \"fake-filler\", \"exact\", \"{{image}}\", \"{{mask}}\", \"{{out}}\"]\nprofile = \"grey\"\ntimeout_s = 30\nrevision = \"v18-scripted\"\nspend_usd_per_call = {cost}\n",
            binary.to_str().unwrap()
        )
    };
    std::fs::write(&config, configuration(0.04)).unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = tool.join("source.png");
    std::fs::write(&input, include_bytes!("../ui/tests/fixtures/panorama.png")).unwrap();
    let asset = assets.import_path(input).await.unwrap();
    let c = Arc::new(
        Coordinator::open(
            assets.clone(),
            vec![Descriptor {
                id: "rascal".into(),
                executable: binary.clone(),
                config: Some(config.clone()),
                args: vec![
                    "serve".into(),
                    "--workspace".into(),
                    tool.join("workspace").display().to_string(),
                    "--runs".into(),
                    tool.join("runs").display().to_string(),
                ],
                cwd: tool.clone(),
                allow_dirs: vec![],
                evidence_dirs: vec![tool],
            }],
        )
        .await
        .unwrap(),
    );
    peer::run(&c, async {
        let description = c.description("rascal").await.unwrap();
        let seam = description.document["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == "seam")
            .unwrap();
        assert_eq!(seam["readiness"]["state"], "ready", "{seam}");
        assert_eq!(seam["cost"]["may_spend"], true);
        let request = |id: &str, cap| {
            serde_json::from_value::<wafflery::library::attempts::Submission>(json!({
                "id": id, "tool": "rascal", "action": "seam", "revision": description.revision,
                "params": {"filler":"local", "projection":"equirectangular", "max_spend_usd":cap},
                "images": {"image":[asset.id]},
                "frames": {"image":[{"projection":"equirectangular","topology":"wrap"}]}
            }))
            .unwrap()
        };
        let above_config = request("above-configured-cap", 0.2);
        c.prepare(above_config.clone()).await.unwrap();
        assert!(c.estimate_prepared(&above_config.id).await.is_err());
        assert!(c.submit_prepared(above_config).await.is_err());
        let low = request("cap-refused", 0.01);
        c.prepare(low.clone()).await.unwrap();
        let quote = c.estimate_prepared(&low.id).await.unwrap();
        assert_eq!(quote.estimate["refused"], "cap");
        assert!(c.submit_prepared(low).await.is_err());
        assert_eq!(c.snapshot().await.unwrap()["attempts"], json!([]));
        let allowed = request("scripted-paid", 0.05);
        c.prepare(allowed.clone()).await.unwrap();
        let quote = c.estimate_prepared(&allowed.id).await.unwrap();
        assert_eq!(quote.estimate["allowed"], true);
        assert_eq!(quote.estimate["worst_case_usd"], 0.04);
        assert_eq!(c.snapshot().await.unwrap()["attempts"], json!([]));
        c.submit_prepared(allowed.clone()).await.unwrap();
        let saved = peer::settled(&c, &allowed.id).await;
        assert_eq!(saved.execution, "completed", "{:?}", saved.result);
        assert_eq!(
            saved.preservation["contract"], "complete",
            "{:?}",
            saved.notices
        );
        assert_eq!(saved.result.as_ref().unwrap()["spend"]["usd"], 0.04);
        assert_eq!(
            saved.wire.as_ref().unwrap()["params"]["_meta"]["table-manners/estimate"],
            saved.observations["estimate"]["binding"]
        );
        assert!(
            saved.observations["estimate"]["binding"]
                .as_str()
                .unwrap()
                .starts_with("sha256:")
        );
        assert!(
            quote.estimate.get("binding").is_none(),
            "opaque bindings stay out of the UI quote"
        );
        let session = c.session("rascal").await.unwrap();
        assert_eq!(session["spent_usd"], 0.04);
        assert_eq!(session["reserved_usd"], 0.0);
        let over_budget = request("budget-refused", 0.05);
        c.prepare(over_budget.clone()).await.unwrap();
        assert_eq!(
            c.estimate_prepared(&over_budget.id).await.unwrap().estimate["refused"],
            "budget"
        );
        assert!(c.submit_prepared(over_budget.clone()).await.is_err());
        std::fs::write(&config, configuration(0.03)).unwrap();
        c.refresh("rascal").await.unwrap();
        let unchanged = c.estimate_prepared(&over_budget.id).await.unwrap();
        assert_eq!(
            unchanged.estimate["effective"]["config_identity"],
            quote.estimate["effective"]["config_identity"]
        );
        assert_eq!(unchanged.estimate["worst_case_usd"], 0.04);
        assert!(c.restart("rascal", false).await.is_err());
        let replacement = c.restart("rascal", true).await.unwrap();
        assert_ne!(
            replacement.tool_session["id"],
            description.tool_session["id"]
        );
        let mut after = request("after-restart", 0.05);
        after.revision = replacement.revision;
        c.prepare(after.clone()).await.unwrap();
        let repriced = c.estimate_prepared(&after.id).await.unwrap();
        assert_eq!(repriced.estimate["worst_case_usd"], 0.03);
        assert_ne!(
            repriced.estimate["effective"]["config_identity"],
            quote.estimate["effective"]["config_identity"]
        );
        assert_eq!(c.session("rascal").await.unwrap()["spent_usd"], 0.0);
        assert!(c.prepared_estimate(&over_budget.id).await.is_err());
        assert_eq!(
            serde_json::to_value(c.attempts.get(&saved.id).await.unwrap()).unwrap(),
            serde_json::to_value(saved).unwrap()
        );
    })
    .await;
}
