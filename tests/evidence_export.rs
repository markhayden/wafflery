use serde_json::json;
use std::io::Read;
use wafflehouse::library::{
    assets::AssetStore,
    attempts::{AttemptStore, Submission},
    export::archive,
};
#[tokio::test]
async fn evidence_archive_contains_verified_bytes_and_only_safe_entries() {
    let dir = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
    let image = dir.path().join("original.png");
    std::fs::write(&image, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(image.clone()).await.unwrap();
    let attempts = AttemptStore::new(assets.database.clone());
    let request:Submission=serde_json::from_value(json!({"id":"../hostile","tool":"sample","action":"mark","revision":"r","params":{"label":"../../unsafe"},"images":{"../../source":[asset.id]}})).unwrap();
    attempts
        .create(request, json!({}), json!({}), json!({}))
        .await
        .unwrap();
    let saved = attempts
        .update("../hostile", |a| {
            a.execution = "completed".into();
            a.preservation["contract"] = json!("complete");
            Ok(())
        })
        .await
        .unwrap();
    let export = archive(&assets, saved.clone(), true).await.unwrap();
    assert!(export.size > 0);
    let listing = std::process::Command::new("/usr/bin/tar")
        .arg("-tzf")
        .arg(export.file.path())
        .output()
        .unwrap();
    assert!(listing.status.success());
    assert!(
        String::from_utf8(listing.stdout)
            .unwrap()
            .contains("manifest.json")
    );
    let gzip = flate2::read::GzDecoder::new(export.file.reopen().unwrap());
    let mut tar = tar::Archive::new(gzip);
    let mut files = std::collections::BTreeMap::new();
    for entry in tar.entries().unwrap() {
        let mut entry = entry.unwrap();
        assert!(entry.header().entry_type().is_file());
        let name = entry.path().unwrap().to_str().unwrap().to_owned();
        assert!(!name.starts_with('/') && !name.contains(".."));
        let mut bytes = vec![];
        entry.read_to_end(&mut bytes).unwrap();
        files.insert(name, bytes);
    }
    let manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        use sha2::{Digest, Sha256};
        let bytes = &files[file["path"].as_str().unwrap()];
        assert_eq!(
            Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            file["sha256"]
        );
    }
    assert_eq!(files.len(), 3);
    assert_eq!(
        std::fs::read(image).unwrap(),
        include_bytes!("../ui/tests/fixtures/sample.png")
    );
    let path = assets
        .root()
        .join("assets/sha256")
        .join(&asset.sha256[..2])
        .join(&asset.sha256);
    std::fs::remove_file(path).unwrap();
    assert!(archive(&assets, saved.clone(), true).await.is_err());
    assert!(archive(&assets, saved, false).await.is_ok());
    assets.database.shutdown().await.unwrap();
}
