//! Compressing and decompressing bytes (spec 14.21, decision 0103): gzip
//! and deflate through `flate2` on its pure-Rust backend, zstd, brotli
//! and lz4 (its frame format), the same in the interpreter and native
//! code. A format is its number, the index of its variant in
//! `std.compress`'s `Format`.

use std::io::{Read, Write};

/// `Format.Gzip` to `Format.Lz4`.
pub const GZIP: i64 = 0;
pub const DEFLATE: i64 = 1;
pub const ZSTD: i64 = 2;
pub const BROTLI: i64 = 3;
pub const LZ4: i64 = 4;

/// Why data couldn't be decoded.
#[derive(Debug, PartialEq)]
pub enum Fail {
    /// More than `most` bytes.
    TooLarge,
    /// Not in the format, or cut short.
    Corrupt(String),
}

/// `data` compressed in `format` at `level`, which the caller checked is
/// in the format's range (`std.compress` panics otherwise).
pub fn encode(data: &[u8], format: i64, level: i64) -> Vec<u8> {
    let written = match format {
        GZIP | DEFLATE => {
            let level = flate2::Compression::new(level.clamp(0, 9) as u32);
            if format == GZIP {
                let mut w = flate2::write::GzEncoder::new(Vec::new(), level);
                w.write_all(data).and_then(|()| w.finish())
            } else {
                let mut w = flate2::write::DeflateEncoder::new(Vec::new(), level);
                w.write_all(data).and_then(|()| w.finish())
            }
        }
        #[cfg(feature = "zstd")]
        ZSTD => zstd::bulk::compress(data, level.clamp(1, 22) as i32),
        #[cfg(not(feature = "zstd"))]
        ZSTD => Err(std::io::Error::other("zstd isn't in this build")),
        BROTLI => {
            let mut out = Vec::new();
            {
                let quality = level.clamp(0, 11) as u32;
                let mut w = brotli::CompressorWriter::new(&mut out, 4096, quality, 22);
                w.write_all(data).map(|()| drop(w))
            }
            .map(|()| out)
        }
        _ => {
            let mut w = lz4_flex::frame::FrameEncoder::new(Vec::new());
            w.write_all(data)
                .map_err(std::io::Error::other)
                .and_then(|()| w.finish().map_err(std::io::Error::other))
        }
    };
    // Writing into memory can't fail; the encoders' own errors are bugs.
    written.unwrap_or_else(|e| panic!("compressing failed: {e}"))
}

/// The bytes `data` holds in `format`: at most `most`.
pub fn decode(data: &[u8], format: i64, most: usize) -> Result<Vec<u8>, Fail> {
    let reader: Box<dyn Read + '_> = match format {
        GZIP => Box::new(flate2::read::MultiGzDecoder::new(data)),
        DEFLATE => Box::new(flate2::read::DeflateDecoder::new(data)),
        #[cfg(feature = "zstd")]
        ZSTD => Box::new(zstd::stream::read::Decoder::new(data).map_err(corrupt)?),
        #[cfg(not(feature = "zstd"))]
        ZSTD => return Err(Fail::Corrupt("zstd isn't in this build".into())),
        BROTLI => Box::new(brotli::Decompressor::new(data, 4096)),
        _ => Box::new(lz4_flex::frame::FrameDecoder::new(data)),
    };
    let mut out = Vec::new();
    // One byte past `most` tells a body of exactly `most` from a larger one.
    let limit = (most as u64).saturating_add(1);
    reader.take(limit).read_to_end(&mut out).map_err(corrupt)?;
    if out.len() > most {
        return Err(Fail::TooLarge);
    }
    Ok(out)
}

fn corrupt(e: std::io::Error) -> Fail {
    Fail::Corrupt(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_format_gives_back_what_it_took() {
        let text = "Hello, World! ".repeat(1000).into_bytes();
        for format in [GZIP, DEFLATE, ZSTD, BROTLI, LZ4] {
            let packed = encode(&text, format, if format == ZSTD { 3 } else { 1 });
            assert!(packed.len() < text.len() / 10, "format {format}");
            assert_eq!(decode(&packed, format, text.len()).unwrap(), text);
            assert_eq!(decode(&packed, format, text.len() - 1), Err(Fail::TooLarge));
            assert!(matches!(
                decode(b"not packed", format, 100),
                Err(Fail::Corrupt(_))
            ));
        }
    }

    #[test]
    fn gzip_members_one_after_another_decode_as_one() {
        let mut both = encode(b"one ", GZIP, 6);
        both.extend(encode(b"two", GZIP, 6));
        assert_eq!(decode(&both, GZIP, 100).unwrap(), b"one two");
    }
}
