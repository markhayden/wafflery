use std::io::Write;
use wafflery::library::assets::AssetStore;

fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend((data.len() as u32).to_be_bytes());
    output.extend(kind);
    output.extend(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    output.extend(crc.finalize().to_be_bytes());
}
fn interlaced(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = vec![];
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 2, 0, 0, 1]);
    chunk(&mut bytes, b"IHDR", &header);
    let mut data = vec![];
    for (x0, y0, dx, dy) in [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ] {
        for y in (y0..height).step_by(dy) {
            data.push(0);
            for x in (x0..width).step_by(dx) {
                data.extend([(x * 255 / width) as u8, (y * 255 / height) as u8, 100]);
            }
        }
    }
    let mut encoded = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::fast());
    encoded.write_all(&data).unwrap();
    chunk(&mut bytes, b"IDAT", &encoded.finish().unwrap());
    chunk(&mut bytes, b"IEND", &[]);
    bytes
}
#[tokio::test]
async fn streamed_png_handles_adam7_palette_transparency_and_sixteen_bit_samples() {
    let root = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&root.path().join("Library")).unwrap();
    let mut cases = vec![interlaced(17, 11)];
    for (color, depth, palette, trns, data) in [
        (
            png::ColorType::Indexed,
            png::BitDepth::One,
            vec![20, 80, 220, 150, 0, 60],
            vec![128, 255],
            vec![0b0100_0000],
        ),
        (
            png::ColorType::GrayscaleAlpha,
            png::BitDepth::Eight,
            vec![],
            vec![],
            vec![20, 128, 100, 255],
        ),
        (
            png::ColorType::Grayscale,
            png::BitDepth::Sixteen,
            vec![],
            vec![],
            vec![20, 0, 100, 0],
        ),
    ] {
        let mut bytes = vec![];
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(color);
            encoder.set_depth(depth);
            if !palette.is_empty() {
                encoder.set_palette(palette);
            }
            if !trns.is_empty() {
                encoder.set_trns(trns);
            }
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&data)
                .unwrap();
        }
        cases.push(bytes);
    }
    for (index, bytes) in cases.into_iter().enumerate() {
        let expected = image::load_from_memory(&bytes).unwrap().into_rgba8();
        let source = root.path().join(format!("image-{index}.png"));
        std::fs::write(&source, &bytes).unwrap();
        let asset = store.import_path(source).await.unwrap();
        let actual = image::load_from_memory(&store.thumbnail(&asset.id).await.unwrap())
            .unwrap()
            .into_rgba8();
        assert_eq!(actual, expected, "codec case {index}");
        assert_eq!(store.read(&asset.id).await.unwrap().1, bytes);
    }
    store.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_corrupt_png_compressed_checksum_is_not_ready_even_with_valid_chunk_crcs() {
    let root = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&root.path().join("Library")).unwrap();
    let mut bytes = interlaced(17, 11);
    let mut start = 8;
    while start < bytes.len() {
        let size = u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap()) as usize;
        if &bytes[start + 4..start + 8] == b"IDAT" {
            bytes[start + 8 + size - 1] ^= 1;
            let crc = crc32fast::hash(&bytes[start + 4..start + 8 + size]);
            bytes[start + 8 + size..start + 12 + size].copy_from_slice(&crc.to_be_bytes());
            break;
        }
        start += size + 12;
    }
    let source = root.path().join("bad.png");
    std::fs::write(&source, bytes).unwrap();
    assert!(store.import_path(source).await.is_err());
    assert!(store.list().await.unwrap().is_empty());
    store.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn interlaced_downsampling_matches_independent_full_decode_within_sampling_precision() {
    let root = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&root.path().join("Library")).unwrap();
    let bytes = interlaced(2049, 1025);
    let reference = image::load_from_memory(&bytes)
        .unwrap()
        .resize(512, 512, image::imageops::FilterType::Triangle)
        .into_rgba8();
    let path = root.path().join("large.png");
    std::fs::write(&path, bytes).unwrap();
    let asset = store.import_path(path).await.unwrap();
    let preview = image::load_from_memory(&store.thumbnail(&asset.id).await.unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!(preview.dimensions(), reference.dimensions());
    for (a, b) in preview.as_raw().iter().zip(reference.as_raw()) {
        assert!(a.abs_diff(*b) <= 2);
    }
    store.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn jpeg_thumbnail_honors_exif_rotation_without_changing_original_dimensions_or_bytes() {
    let root = tempfile::tempdir().unwrap();
    let store = AssetStore::open(&root.path().join("Library")).unwrap();
    let path = root.path().join("rotate.jpg");
    image::RgbImage::from_pixel(32, 16, image::Rgb([20u8, 100, 240]))
        .save(&path)
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    let mut exif =
        b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0".to_vec();
    let mut bytes = vec![255, 216, 255, 225];
    bytes.extend((exif.len() as u16 + 2).to_be_bytes());
    bytes.append(&mut exif);
    bytes.extend(&original[2..]);
    std::fs::write(&path, &bytes).unwrap();
    let asset = store.import_path(path).await.unwrap();
    let thumbnail = image::load_from_memory(&store.thumbnail(&asset.id).await.unwrap()).unwrap();
    assert_eq!((thumbnail.width(), thumbnail.height()), (16, 32));
    assert_eq!((asset.width, asset.height), (32, 16));
    assert_eq!(store.read(&asset.id).await.unwrap().1, bytes);
    store.database.shutdown().await.unwrap();
}
