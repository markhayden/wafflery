//! Opt-in, no credentials, no config, no filler. Not part of the required foundation gate.
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use wafflehouse::{
    library::assets::AssetStore,
    tools::{coordinator::Coordinator, registration::Descriptor},
};
#[tokio::test]
#[ignore = "requires WAFFLEHOUSE_RASCAL_BIN pointing to an explicitly selected local binary"]
async fn real_rascal_offline_seam_uses_the_same_dispatch_and_evidence_guards() {
    let Some(binary) = std::env::var_os("WAFFLEHOUSE_RASCAL_BIN").map(PathBuf::from) else {
        eprintln!("SKIPPED: WAFFLEHOUSE_RASCAL_BIN not set");
        return;
    };
    assert!(binary.is_absolute(), "Choose an absolute binary path");
    if !binary.is_file() {
        eprintln!("SKIPPED: selected Rascal binary is absent");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir(&tool).unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let input = dir.path().join("owned-source.png");
    let bytes = include_bytes!("../ui/tests/fixtures/panorama.png");
    std::fs::write(&input, bytes).unwrap();
    let asset = assets.import_path(input.clone()).await.unwrap();
    let descriptor = Descriptor {
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
        roots: vec![tool],
    };
    let c = Arc::new(Coordinator::open(assets, vec![descriptor]).await.unwrap());
    let description = match c.description("real-offline-smoke").await {
        Ok(d) => d,
        Err(_) => {
            let state = c.snapshot().await.unwrap();
            let message = state["tools"][0]["error"]
                .as_str()
                .unwrap_or("Unavailable")
                .to_owned();
            assert!(c.close().await);
            c.assets.database.shutdown().await.unwrap();
            if message.contains("Unsupported table-manners") {
                eprintln!("SKIPPED: {message}");
                return;
            }
            panic!("Real Rascal failed description: {message}");
        }
    };
    eprintln!(
        "Selected {} — tool {} {}, table-manners {}",
        binary.display(),
        description.document["tool"]["name"],
        description.document["tool"]["version"],
        description.document["contract"]
    );
    let request=serde_json::from_value(json!({"id":"offline-seam","tool":"real-offline-smoke","action":"seam","revision":description.revision,"params":{"projection":"equirectangular"},"images":{"image":[asset.id]},"frames":{"image":[{"projection":"equirectangular","topology":"wrap"}]}})).unwrap();
    let outcome = async {
        c.submit(request).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let attempt = c.attempts.get("offline-seam").await.unwrap();
                if attempt.preservation["contract"] != "pending" {
                    break attempt;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("bounded offline smoke")
    }
    .await;
    assert!(c.close().await);
    c.assets.database.shutdown().await.unwrap();
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
