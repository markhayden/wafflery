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

#[tokio::test]
async fn thumbnails_are_bounded_cached_and_never_replace_originals() {
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    for extension in ["png", "jpg"] {
        let source = temp.path().join(format!("large.{extension}"));
        image::RgbImage::from_pixel(2048, 1024, image::Rgb([30, 120, 240]))
            .save(&source)
            .unwrap();
        let original = std::fs::read(&source).unwrap();
        let asset = store.import_path(source).await.unwrap();
        let thumbnail = store.thumbnail(&asset.id).await.unwrap();
        let decoded = image::load_from_memory(&thumbnail).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (512, 256));
        assert_eq!(store.read(&asset.id).await.unwrap().1, original);
        assert_eq!((asset.width, asset.height), (2048, 1024));
        assert_eq!(store.thumbnail(&asset.id).await.unwrap(), thumbnail);
    }
    store.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn derived_cache_recovers_from_missing_corrupt_or_symlinked_files_and_cleans_up() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    use wafflery::library::cleanup::{Cleanup, Selection};
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    let source = temp.path().join("source.png");
    png(&source, 50);
    let asset = store.import_path(source).await.unwrap();
    let cache = store
        .root()
        .join("assets/sha256")
        .join(&asset.sha256[..2])
        .join(format!("{}.thumbnail.png", asset.sha256));
    let expected = store.thumbnail(&asset.id).await.unwrap();
    std::fs::remove_file(&cache).unwrap();
    assert_eq!(store.thumbnail(&asset.id).await.unwrap(), expected);
    std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&cache, b"broken").unwrap();
    assert_eq!(store.thumbnail(&asset.id).await.unwrap(), expected);
    let outside = temp.path().join("outside");
    std::fs::write(&outside, b"never follow this").unwrap();
    std::fs::remove_file(&cache).unwrap();
    symlink(&outside, &cache).unwrap();
    assert_eq!(store.thumbnail(&asset.id).await.unwrap(), expected);
    assert_eq!(std::fs::read(&outside).unwrap(), b"never follow this");
    let selection = Selection {
        unused_assets: true,
        ..Default::default()
    };
    let cleanup = Cleanup(store.clone());
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    cleanup.confirm(selection, preview.token).await.unwrap();
    assert!(!cache.exists());
    assert!(store.thumbnail(&asset.id).await.is_err());
    store.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn thumbnail_decode_keeps_alpha_grayscale_and_complete_file_validation() {
    let temp = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temp.path().join("Library")).unwrap();
    let rgba = temp.path().join("alpha.png");
    image::RgbaImage::from_pixel(8, 4, image::Rgba([20, 100, 240, 128]))
        .save(&rgba)
        .unwrap();
    let asset = store.import_path(rgba.clone()).await.unwrap();
    let decoded = image::load_from_memory(&store.thumbnail(&asset.id).await.unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (8, 4));
    assert_eq!(decoded.get_pixel(0, 0).0, [20, 100, 240, 128]);
    let gray = temp.path().join("gray.jpg");
    image::GrayImage::from_pixel(1600, 800, image::Luma([120]))
        .save(&gray)
        .unwrap();
    let asset = store.import_path(gray.clone()).await.unwrap();
    let decoded = image::load_from_memory(&store.thumbnail(&asset.id).await.unwrap())
        .unwrap()
        .into_rgb8();
    assert_eq!(decoded.get_pixel(0, 0).0, [120, 120, 120]);
    for source in [rgba, gray] {
        let original = std::fs::read(&source).unwrap();
        std::fs::write(&source, &original[..original.len() / 2]).unwrap();
        assert!(store.import_path(source).await.is_err());
    }
    assert_eq!(store.list().await.unwrap().len(), 2);
    store.database.shutdown().await.unwrap();
}
