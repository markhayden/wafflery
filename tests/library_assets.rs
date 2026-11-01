use wafflery::library::assets::AssetStore;

fn png(path: &std::path::Path, color: u8) {
    image::RgbImage::from_pixel(3, 2, image::Rgb([color, 120, 240]))
        .save(path)
        .unwrap();
}

#[tokio::test]
async fn imports_are_immutable_deduplicated_and_survive_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Library");
    let source = temp.path().join("source.png");
    png(&source, 10);
    let original = std::fs::read(&source).unwrap();
    let store = AssetStore::open(&root).unwrap();
    let first = store.import_path(source.clone()).await.unwrap();
    let duplicate = store.import_path(source.clone()).await.unwrap();
    assert_eq!(first.id, duplicate.id);
    assert_eq!(
        (first.width, first.height, first.media_type.as_str()),
        (3, 2, "image/png")
    );
    png(&source, 20);
    assert_eq!(store.read(&first.id).await.unwrap().1, original);
    assert_ne!(store.import_path(source).await.unwrap().id, first.id);
    store.database.shutdown().await.unwrap();
    let reopened = AssetStore::open(&root).unwrap();
    assert_eq!(reopened.read(&first.id).await.unwrap().1, original);
    assert_eq!(reopened.list().await.unwrap().len(), 2);
    reopened.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_content_and_database_failure_never_create_an_asset_record() {
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    let source = temp.path().join("disguised.png");
    for bytes in [
        b"<html><script>alert(1)</script>".as_slice(),
        b"<svg onload='alert(1)'/>",
        b"\x89PNG\r\n\x1a\n",
    ] {
        std::fs::write(&source, bytes).unwrap();
        assert!(store.import_path(source.clone()).await.is_err());
    }
    png(&source, 10);
    store
        .database
        .call(|connection| {
            connection.pragma_update(None, "query_only", true)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(store.import_path(source).await.is_err());
    assert!(store.list().await.unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(store.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    store.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn corrupt_owned_bytes_are_neither_served_nor_overwritten() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    let source = temp.path().join("source.png");
    png(&source, 10);
    let asset = store.import_path(source.clone()).await.unwrap();
    let owned = store
        .root()
        .join("assets/sha256")
        .join(&asset.sha256[..2])
        .join(&asset.sha256);
    std::fs::set_permissions(&owned, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&owned, "damaged").unwrap();
    assert!(store.read(&asset.id).await.is_err());
    assert!(store.import_path(source).await.is_err());
    assert_eq!(std::fs::read_to_string(owned).unwrap(), "damaged");
    assert!(store.read("../../outside").await.is_err());
    store.database.shutdown().await.unwrap();
}
