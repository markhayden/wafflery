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
            "--allow-dir".into(),
            assets.root().join("execution").display().to_string(),
            "--workspace".into(),
            tool.join("workspace").display().to_string(),
            "--runs".into(),
            tool.join("runs").display().to_string(),
        ],
        cwd: tool.clone(),
        roots: vec![tool],
    };
    let c = Arc::new(Coordinator::open(assets, vec![descriptor]).await.unwrap());
    let outcome = peer::run(&c, async {
        let description = c.description("real-offline-smoke").await.unwrap();
        assert_eq!(description.document["contract"], "0.3.0");
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
