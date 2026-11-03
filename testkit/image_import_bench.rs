//! Opt-in import measurements. Input generation is separate so peak RSS measures import only.
use std::{path::PathBuf, time::Instant};
use wafflery::library::assets::AssetStore;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let source = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("Select an owned benchmark image"),
    );
    if source == std::path::Path::new("--generate") {
        let root = PathBuf::from(
            std::env::args_os()
                .nth(2)
                .expect("Select a disposable fixture folder"),
        );
        std::fs::create_dir_all(&root).unwrap();
        for (width, height, name) in [
            (2048, 1024, "studio"),
            (15000, 7500, "panorama"),
            (2048, 1024, "texture"),
            (15000, 7500, "panorama-texture"),
        ] {
            let image = image::RgbImage::from_fn(width, height, |x, y| {
                if name.contains("texture") {
                    let block = (x / 4).wrapping_mul(0x9e3779b9) ^ (y / 4).wrapping_mul(0x85ebca6b);
                    let mixed = (block ^ (block >> 16)).wrapping_mul(0xc2b2ae35);
                    image::Rgb([(mixed >> 24) as u8, (mixed >> 16) as u8, (mixed >> 8) as u8])
                } else {
                    image::Rgb([(x * 255 / width) as u8, (y * 255 / height) as u8, 130])
                }
            });
            image.save(root.join(format!("{name}.png"))).unwrap();
            image.save(root.join(format!("{name}.jpg"))).unwrap();
        }
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&temporary.path().join("Library")).unwrap();
    let start = Instant::now();
    let asset = store.import_path(source).await.unwrap();
    let import_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let (_, bytes) = store.read(&asset.id).await.unwrap();
    let original_read_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let thumbnail = store.thumbnail(&asset.id).await.unwrap();
    let thumbnail_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        serde_json::json!({"width":asset.width,"height":asset.height,"format":asset.media_type,"encoded_bytes":bytes.len(),"import_ms":import_ms,"original_read_ms":original_read_ms,"thumbnail_ms":thumbnail_ms,"thumbnail_bytes":thumbnail.len()})
    );
    store.database.shutdown().await.unwrap();
}
