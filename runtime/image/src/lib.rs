//! Images for `std.image` (spec 14.29, decision 0117): PNG and JPEG files
//! decoded to RGBA pixels, 8 bits a part, row by row from the top, and
//! PNG written. A decoded image is kept by number until `std.image` has
//! read its size and pixels.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(test)]
mod tests;

/// A picture: its size, and four bytes a pixel (red, green, blue,
/// opacity), row by row from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The picture in a PNG or JPEG file's bytes, told apart by their first
/// bytes; or why there is none.
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        png_image(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        jpeg_image(bytes)
    } else {
        Err("this is not a PNG or JPEG file".into())
    }
}

fn png_image(bytes: &[u8]) -> Result<Image, String> {
    let bad = |e: png::DecodingError| format!("this PNG file can't be read: {e}");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Palettes and low bit depths widened, 16-bit parts cut to 8.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(bad)?;
    let size = reader
        .output_buffer_size()
        .ok_or("this PNG file is too big")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(bad)?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::Indexed => return Err("this PNG file's palette can't be read".into()),
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn jpeg_image(bytes: &[u8]) -> Result<Image, String> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    let pixels = decoder
        .decode()
        .map_err(|e| format!("this JPEG file can't be read: {e}"))?;
    let info = decoder.info().ok_or("this JPEG file has no size")?;
    use jpeg_decoder::PixelFormat as F;
    let rgba = match info.pixel_format {
        F::RGB24 => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        F::L8 => pixels.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        // Two bytes a pixel, the high one first.
        F::L16 => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], 255])
            .collect(),
        F::CMYK32 => (pixels.as_chunks::<4>().0.iter())
            .flat_map(|p| {
                let k = 255 - u16::from(p[3]);
                let part = |c: u8| ((255 - u16::from(c)) * k / 255) as u8;
                [part(p[0]), part(p[1]), part(p[2]), 255]
            })
            .collect(),
    };
    Ok(Image {
        width: u32::from(info.width),
        height: u32::from(info.height),
        rgba,
    })
}

/// `rgba` (`width` × `height` pixels, four bytes each) as a PNG file.
pub fn png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    if rgba.len() as u64 != u64::from(width) * u64::from(height) * 4 || width == 0 || height == 0 {
        return Err(format!(
            "{width} × {height} pixels take {} bytes",
            u64::from(width) * u64::from(height) * 4
        ));
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

fn kept() -> MutexGuard<'static, (HashMap<i64, Image>, i64)> {
    static KEPT: OnceLock<Mutex<(HashMap<i64, Image>, i64)>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Decodes `bytes` and keeps the picture: its number.
pub fn decode_kept(bytes: &[u8]) -> Result<i64, String> {
    let img = decode(bytes)?;
    let mut k = kept();
    k.1 += 1;
    let n = k.1;
    k.0.insert(n, img);
    Ok(n)
}

/// Picture `n`'s size; `(0, 0)` once it is gone.
pub fn size(n: i64) -> (i64, i64) {
    kept()
        .0
        .get(&n)
        .map_or((0, 0), |i| (i64::from(i.width), i64::from(i.height)))
}

/// Picture `n`'s pixels, taking them: the picture is let go.
pub fn take_pixels(n: i64) -> Vec<u8> {
    kept().0.remove(&n).map(|i| i.rgba).unwrap_or_default()
}
