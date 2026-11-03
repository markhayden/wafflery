//! Derived previews only: validate encoded input while keeping the original bytes authoritative.
use super::{Error, Result};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, RgbaImage};
use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
    path::Path,
};
pub(super) const EDGE: u32 = 512;
const PIXELS: u64 = 128_000_000;
pub(super) struct Preview {
    pub width: u32,
    pub height: u32,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
}
fn dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > PIXELS {
        return Err(Error::InvalidAsset(
            "image exceeds 128 megapixels or has invalid dimensions",
        ));
    }
    Ok(())
}
fn bounds(width: u32, height: u32, edge: u32) -> (u32, u32) {
    let scale = f64::from(edge.min(width.max(height))) / f64::from(width.max(height));
    (
        (f64::from(width) * scale).round().max(1.0) as u32,
        (f64::from(height) * scale).round().max(1.0) as u32,
    )
}
pub(super) fn decode(path: &Path) -> Result<Preview> {
    let mut file = File::open(path)?;
    let mut signature = [0; 8];
    let size = file.read(&mut signature)?;
    file.rewind()?;
    let (image, width, height, media_type) = if size == 8 && signature == *b"\x89PNG\r\n\x1a\n" {
        png(BufReader::new(file))?
    } else if signature[..2] == [255, 216] {
        jpeg(BufReader::new(file))?
    } else {
        return Err(Error::InvalidAsset(
            "only PNG and JPEG images are supported",
        ));
    };
    let mut output = Cursor::new(Vec::new());
    let (tw, th) = bounds(image.width(), image.height(), EDGE);
    image
        .resize(tw, th, image::imageops::FilterType::Triangle)
        .write_to(&mut output, ImageFormat::Png)?;
    Ok(Preview {
        width,
        height,
        media_type,
        bytes: output.into_inner(),
    })
}
fn png(file: BufReader<File>) -> Result<(DynamicImage, u32, u32, &'static str)> {
    let bad = || Error::InvalidAsset("Choose a complete PNG image within the image limits.");
    let mut decoder = png::Decoder::new_with_limits(
        file,
        png::Limits {
            bytes: 64 * 1024 * 1024,
        },
    );
    decoder.ignore_checksums(false);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| bad())?;
    let (width, height, interlaced) = (
        reader.info().width,
        reader.info().height,
        reader.info().interlaced,
    );
    dimensions(width, height)?;
    let (color, depth) = reader.output_color_type();
    if depth != png::BitDepth::Eight {
        return Err(bad());
    }
    let channels = color.samples();
    let (sw, sh) = bounds(width, height, EDGE * 2);
    let mut small = RgbaImage::new(sw, sh);
    let source_y: Vec<u32> = (0..sh)
        .map(|y| (u64::from(y) * u64::from(height) / u64::from(sh)) as u32)
        .collect();
    // Each selected source pixel occurs in exactly one pass. No full-resolution raster is allocated.
    let passes = if interlaced {
        vec![
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ]
    } else {
        vec![(0, 0, 1, 1)]
    };
    for (pass, (x0, y0, dx, dy)) in passes.into_iter().enumerate() {
        if width <= x0 || height <= y0 {
            continue;
        }
        let row_width = (width - x0).div_ceil(dx);
        for (line, y) in (y0..height).step_by(dy as usize).enumerate() {
            let row = reader
                .next_interlaced_row()
                .map_err(|_| bad())?
                .ok_or_else(bad)?;
            if interlaced
                && !matches!(row.interlace(),png::InterlaceInfo::Adam7(info) if *info==png::Adam7Info::new(pass as u8+1,line as u32,width))
            {
                return Err(bad());
            }
            if row.data().len() != row_width as usize * channels {
                return Err(bad());
            }
            if let Ok(ty) = source_y.binary_search(&y) {
                let ty = ty as u32;
                for tx in 0..sw {
                    let x = (u64::from(tx) * u64::from(width) / u64::from(sw)) as u32;
                    if x < x0 || !(x - x0).is_multiple_of(dx) {
                        continue;
                    }
                    let offset = ((x - x0) / dx) as usize * channels;
                    let sample = &row.data()[offset..offset + channels];
                    let rgba = match color {
                        png::ColorType::Grayscale => [sample[0], sample[0], sample[0], 255],
                        png::ColorType::GrayscaleAlpha => {
                            [sample[0], sample[0], sample[0], sample[1]]
                        }
                        png::ColorType::Rgb => [sample[0], sample[1], sample[2], 255],
                        png::ColorType::Rgba => [sample[0], sample[1], sample[2], sample[3]],
                        _ => return Err(bad()),
                    };
                    small.put_pixel(tx, ty, image::Rgba(rgba));
                }
            }
        }
    }
    if reader.next_row().map_err(|_| bad())?.is_some() {
        return Err(bad());
    }
    reader.finish().map_err(|_| bad())?;
    let mut image: DynamicImage = small.into();
    if let Some(orientation) = reader
        .info()
        .exif_metadata
        .as_deref()
        .and_then(image::metadata::Orientation::from_exif_chunk)
    {
        image.apply_orientation(orientation);
    }
    Ok((image, width, height, "image/png"))
}
fn jpeg(mut file: BufReader<File>) -> Result<(DynamicImage, u32, u32, &'static str)> {
    let bad = || Error::InvalidAsset("Choose a complete JPEG image within the image limits.");
    jpeg_header(&mut file).map_err(|error| match error {
        Error::Io(ref io) if io.kind() == std::io::ErrorKind::UnexpectedEof => bad(),
        other => other,
    })?;
    file.rewind()?;
    let mut decoder = jpeg_decoder::Decoder::new(file);
    decoder.set_max_decoding_buffer_size(512 * 1024 * 1024);
    decoder.read_info().map_err(|_| bad())?;
    let info = decoder.info().ok_or_else(bad)?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    dimensions(width, height)?;
    let (sw, sh) = bounds(width, height, EDGE);
    decoder.scale(sw as u16, sh as u16).map_err(|_| bad())?;
    let pixels = decoder.decode().map_err(|_| bad())?;
    let scaled = decoder.info().ok_or_else(bad)?;
    let (w, h) = (u32::from(scaled.width), u32::from(scaled.height));
    let mut image = match scaled.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            DynamicImage::ImageLuma8(ImageBuffer::from_raw(w, h, pixels).ok_or_else(bad)?)
        }
        jpeg_decoder::PixelFormat::L16 => return Err(bad()),
        jpeg_decoder::PixelFormat::RGB24 => {
            DynamicImage::ImageRgb8(ImageBuffer::from_raw(w, h, pixels).ok_or_else(bad)?)
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            let rgb = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| {
                    let k = 255 - u16::from(p[3]);
                    [0, 1, 2].map(|c| ((255 - u16::from(p[c])) * k / 255) as u8)
                })
                .collect();
            DynamicImage::ImageRgb8(ImageBuffer::<Rgb<u8>, _>::from_raw(w, h, rgb).ok_or_else(bad)?)
        }
    };
    if let Some(orientation) = decoder
        .exif_data()
        .and_then(image::metadata::Orientation::from_exif_chunk)
    {
        image.apply_orientation(orientation);
    }
    Ok((image, width, height, "image/jpeg"))
}

// The JPEG decoder exposes colour format but not sampling factors. Bound source-sized
// progressive coefficients before its first allocation; its own limit covers only output pixels.
fn jpeg_header(file: &mut (impl Read + Seek)) -> Result<()> {
    let invalid =
        || Error::InvalidAsset("Choose a complete supported JPEG image within the image limits.");
    let mut soi = [0; 2];
    file.read_exact(&mut soi)?;
    if soi != [255, 216] {
        return Err(invalid());
    }
    loop {
        let mut byte = [0];
        file.read_exact(&mut byte)?;
        if byte[0] != 255 {
            return Err(invalid());
        }
        while byte[0] == 255 {
            file.read_exact(&mut byte)?;
        }
        let marker = byte[0];
        if marker == 0 || marker == 0xd8 || marker == 0xda || marker == 0xd9 {
            return Err(invalid());
        }
        if marker == 1 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let mut length = [0; 2];
        file.read_exact(&mut length)?;
        let length = u16::from_be_bytes(length);
        if length < 2 {
            return Err(invalid());
        }
        if [
            0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf,
        ]
        .contains(&marker)
        {
            if ![0xc0, 0xc1, 0xc2].contains(&marker) {
                return Err(Error::InvalidAsset(
                    "Unsupported JPEG encoding; use a PNG or baseline/progressive JPEG copy.",
                ));
            }
            let mut header = [0; 6];
            if length < 8 {
                return Err(invalid());
            }
            file.read_exact(&mut header)?;
            let (_precision, height, width, count) = (
                header[0],
                u32::from(u16::from_be_bytes([header[1], header[2]])),
                u32::from(u16::from_be_bytes([header[3], header[4]])),
                header[5],
            );
            dimensions(width, height)?;
            if ![1, 3, 4].contains(&count) || length != 8 + 3 * u16::from(count) {
                return Err(invalid());
            }
            let mut sampling = vec![];
            for _ in 0..count {
                let mut component = [0; 3];
                file.read_exact(&mut component)?;
                let (h, v) = (u64::from(component[1] >> 4), u64::from(component[1] & 15));
                if !(1..=4).contains(&h) || !(1..=4).contains(&v) {
                    return Err(invalid());
                }
                sampling.push((h, v));
            }
            let progressive = [0xc2, 0xc6, 0xca, 0xce].contains(&marker);
            let hmax = sampling.iter().map(|(h, _)| *h).max().unwrap();
            let vmax = sampling.iter().map(|(_, v)| *v).max().unwrap();
            let coefficients = if progressive {
                let mcus =
                    u64::from(width).div_ceil(8 * hmax) * u64::from(height).div_ceil(8 * vmax);
                sampling.iter().map(|(h, v)| mcus * h * v * 64 * 2).sum()
            } else {
                0
            };
            // Reserve 64 MiB for scaled output, row workers and metadata in the 512 MiB budget.
            if coefficients > 448 * 1024 * 1024 {
                return Err(Error::InvalidAsset(
                    "JPEG exceeds the library decoder memory budget; choose a smaller copy.",
                ));
            }
            return Ok(());
        }
        file.seek(std::io::SeekFrom::Current(i64::from(length) - 2))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header(width: u16, height: u16, sampling: &[u8], marker: u8) -> Vec<u8> {
        let mut bytes = vec![255, 216, 255, marker];
        bytes.extend((8 + 3 * sampling.len() as u16).to_be_bytes());
        bytes.push(8);
        bytes.extend(height.to_be_bytes());
        bytes.extend(width.to_be_bytes());
        bytes.push(sampling.len() as u8);
        for (id, factor) in sampling.iter().enumerate() {
            bytes.extend([id as u8 + 1, *factor, 0]);
        }
        bytes
    }
    #[test]
    fn stuffed_marker_cannot_hide_an_oversized_frame_from_the_budget_guard() {
        let hidden = header(15000, 7500, &[0x11; 4], 0xc2);
        let small = header(160, 100, &[0x11; 3], 0xc0);
        let mut bytes = vec![255, 216, 255, 0];
        bytes.extend((hidden.len() as u16).to_be_bytes());
        bytes.extend(&hidden[2..]);
        bytes.extend(&small[2..]);
        assert!(jpeg_header(&mut Cursor::new(bytes)).is_err());
    }
    #[test]
    fn progressive_budget_accounts_for_sampling_and_refuses_before_coefficients_allocate() {
        assert!(
            jpeg_header(&mut Cursor::new(header(
                15000,
                7500,
                &[0x22, 0x11, 0x11],
                0xc2
            )))
            .is_ok()
        );
        assert!(matches!(
            jpeg_header(&mut Cursor::new(header(
                15000,
                7500,
                &[0x11, 0x11, 0x11],
                0xc2
            ))),
            Err(Error::InvalidAsset(
                "JPEG exceeds the library decoder memory budget; choose a smaller copy."
            ))
        ));
        assert!(matches!(
            jpeg_header(&mut Cursor::new(header(160, 100, &[0x11], 0xc3))),
            Err(Error::InvalidAsset(
                "Unsupported JPEG encoding; use a PNG or baseline/progressive JPEG copy."
            ))
        ));
        for factors in [&[0x11, 0x11][..], &[0x01], &[0x51]] {
            assert!(jpeg_header(&mut Cursor::new(header(160, 100, factors, 0xc2))).is_err());
        }
    }
}
