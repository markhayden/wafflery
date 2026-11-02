use serde_json::json;
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};
use wafflery::{
    library::{
        assets::AssetStore,
        attempts::{AttemptStore, Submission},
    },
    tools::coordinator::Coordinator,
};

fn directory(root: &Path, id: &str, marked: bool) -> PathBuf {
    let path = root.join("execution").join(id);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::set_permissions(
        root.join("execution"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(path.join("input-0.png"), b"partial or complete input copy").unwrap();
    if marked {
        std::fs::write(
            path.join(".wafflery-preparation.json"),
            serde_json::to_vec(&json!({"version":1,"id":id})).unwrap(),
        )
        .unwrap();
        std::fs::set_permissions(
            path.join(".wafflery-preparation.json"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    path
}
#[tokio::test]
async fn startup_removes_only_marked_unclaimed_inputs_and_preserves_recorded_attempts() {
    let temporary = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temporary.path().join("library")).unwrap();
    let orphan = directory(assets.root(), "abandoned", true);
    let saved = directory(assets.root(), "saved", true);
    // A saved attempt is outside preparation recovery even if its marker is unreadable.
    std::fs::set_permissions(
        saved.join(".wafflery-preparation.json"),
        std::fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let public_marker = directory(assets.root(), "public-marker", true);
    std::fs::set_permissions(
        public_marker.join(".wafflery-preparation.json"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let linked_marker = directory(assets.root(), "linked-marker", true);
    std::fs::hard_link(
        linked_marker.join(".wafflery-preparation.json"),
        temporary.path().join("marker-link"),
    )
    .unwrap();
    let legacy = directory(assets.root(), "unmarked", false);
    let unknown = directory(assets.root(), "unknown-file", true);
    std::fs::write(unknown.join("user-document.txt"), b"keep").unwrap();
    let invalid = directory(assets.root(), "invalid-marker", true);
    std::fs::write(invalid.join(".wafflery-preparation.json"), b"{}").unwrap();
    let request: Submission = serde_json::from_value(json!({"id":"saved","tool":"fixture","action":"mark","revision":"r","params":{},"images":{}})).unwrap();
    let attempts = AttemptStore::new(assets.database.clone());
    attempts
        .create(request, json!({}), json!({}), json!({}))
        .await
        .unwrap();
    let c = Coordinator::open(assets.clone(), vec![]).await.unwrap();
    assert!(!orphan.exists());
    for path in [
        saved,
        legacy,
        unknown,
        invalid,
        public_marker,
        linked_marker,
    ] {
        assert!(path.join("input-0.png").exists(), "{}", path.display());
    }
    assert_eq!(
        attempts.history(None, 50, None, None).await.unwrap().len(),
        1
    );
    assert!(c.close().await);
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn startup_never_follows_preparation_symlinks_or_an_execution_root_alias() {
    let temporary = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temporary.path().join("library")).unwrap();
    let external = temporary.path().join("external");
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("original.png"), b"original").unwrap();
    let inside = directory(assets.root(), "linked-file", true);
    std::fs::remove_file(inside.join("input-0.png")).unwrap();
    symlink(external.join("original.png"), inside.join("input-0.png")).unwrap();
    symlink(&external, assets.root().join("execution/linked-directory")).unwrap();
    let c = Coordinator::open(assets.clone(), vec![]).await.unwrap();
    assert!(inside.join("input-0.png").is_symlink());
    assert_eq!(
        std::fs::read(external.join("original.png")).unwrap(),
        b"original"
    );
    assert!(c.close().await);
    drop(c);
    std::fs::rename(
        assets.root().join("execution"),
        assets.root().join("old-execution"),
    )
    .unwrap();
    symlink(&external, assets.root().join("execution")).unwrap();
    assert!(Coordinator::open(assets.clone(), vec![]).await.is_err());
    assert_eq!(
        std::fs::read(external.join("original.png")).unwrap(),
        b"original"
    );
    assets.database.shutdown().await.unwrap();
}
