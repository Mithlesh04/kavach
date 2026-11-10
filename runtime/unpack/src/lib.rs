//! Packed executables (spec 10.5, decision 0081). A packed file is the
//! starter (`kavach-unpack`), then the compressed program, then a
//! trailer:
//!
//! | Bytes | Holds |
//! |---|---|
//! | 8 | `KVPACK01` |
//! | 8 | the method: 1 deflate, 2 xz (one byte, then zeros) |
//! | 8 | the compressed program's length |
//! | 8 | the unpacked program's length |
//! | 8 | a 64-bit FNV-1a hash of the unpacked program |
//!
//! All numbers are little-endian. `kavach build --compress` makes it
//! with `packed`; the starter (`starter/starter.c`, compiled by `build.rs`
//! and embedded here as `STARTER`) reads it, as `trailer` and `unpack`
//! do for `kavach version`.

use std::io::{Read, Write};
use std::path::PathBuf;

/// The starter, compiled for this system by `build.rs`.
pub const STARTER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/starter.bin"));

/// The first bytes of the trailer.
pub const MAGIC: &[u8; 8] = b"KVPACK01";

/// The trailer's length.
pub const TRAILER: usize = 40;

/// How a program is compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// `--compress`: deflate (`miniz_oxide`), quick to unpack.
    Deflate = 1,
    /// `--compress=max`: xz (`lzma-rust2`), the smallest files.
    Xz = 2,
}

/// What a packed file's trailer says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trailer {
    pub method: Method,
    pub packed_len: u64,
    pub unpacked_len: u64,
    pub hash: u64,
}

/// A 64-bit FNV-1a hash: names the unpacked copy in the cache.
pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// `program` compressed by `method`, with its trailer: what goes after
/// the starter.
pub fn pack(program: &[u8], method: Method) -> std::io::Result<Vec<u8>> {
    let mut out = match method {
        Method::Deflate => miniz_oxide::deflate::compress_to_vec(program, 10),
        Method::Xz => {
            let mut out = Vec::new();
            let mut options = lzma_rust2::XzOptions::with_preset(9);
            // The starter's decoder (xz-embedded) checks CRC32.
            options.set_check_sum_type(lzma_rust2::CheckType::Crc32);
            let mut w = lzma_rust2::XzWriter::new(&mut out, options)?;
            w.write_all(program)?;
            w.finish()?;
            out
        }
    };
    let packed_len = out.len() as u64;
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(method as u64).to_le_bytes());
    out.extend_from_slice(&packed_len.to_le_bytes());
    out.extend_from_slice(&(program.len() as u64).to_le_bytes());
    out.extend_from_slice(&hash(program).to_le_bytes());
    Ok(out)
}

/// `program` packed by `method` behind the starter: a packed program.
pub fn packed(program: &[u8], method: Method) -> std::io::Result<Vec<u8>> {
    let mut file = STARTER.to_vec();
    file.extend(pack(program, method)?);
    Ok(file)
}

/// The trailer at the end of `file`, when it is a packed program.
pub fn trailer(file: &[u8]) -> Option<Trailer> {
    let at = file.len().checked_sub(TRAILER)?;
    let t = &file[at..];
    if &t[..8] != MAGIC {
        return None;
    }
    let word = |i: usize| u64::from_le_bytes(t[i..i + 8].try_into().expect("8 bytes"));
    let method = match word(8) {
        1 => Method::Deflate,
        2 => Method::Xz,
        _ => return None,
    };
    let packed_len = word(16);
    (packed_len <= at as u64).then_some(Trailer {
        method,
        packed_len,
        unpacked_len: word(24),
        hash: word(32),
    })
}

/// The program packed in `file`, checked against its length and hash.
pub fn unpack(file: &[u8]) -> Result<Vec<u8>, String> {
    let t = trailer(file).ok_or("not a packed program")?;
    let end = file.len() - TRAILER;
    let packed = &file[end - t.packed_len as usize..end];
    let program = match t.method {
        Method::Deflate => miniz_oxide::inflate::decompress_to_vec(packed)
            .map_err(|e| format!("can't unpack: {e:?}"))?,
        Method::Xz => {
            let mut out = Vec::with_capacity(t.unpacked_len as usize);
            lzma_rust2::XzReader::new(packed, false)
                .read_to_end(&mut out)
                .map_err(|e| format!("can't unpack: {e}"))?;
            out
        }
    };
    if program.len() as u64 != t.unpacked_len || hash(&program) != t.hash {
        return Err("the packed program is damaged".into());
    }
    Ok(program)
}

/// The folder packed programs unpack into (spec 10.5): `KAVACH_UNPACKED`,
/// else the system's cache folder.
pub fn cache_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    if let Some(dir) = var("KAVACH_UNPACKED") {
        return Some(PathBuf::from(dir));
    }
    let base = if cfg!(windows) {
        PathBuf::from(var("LOCALAPPDATA")?).join("Kavach")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(var("HOME")?).join("Library/Caches/kavach")
    } else if let Some(cache) = var("XDG_CACHE_HOME") {
        PathBuf::from(cache).join("kavach")
    } else {
        PathBuf::from(var("HOME")?).join(".cache/kavach")
    };
    Some(base.join("unpacked"))
}

/// The unpacked copy's file name: its hash and length.
pub fn copy_name(t: &Trailer) -> String {
    let ext = if cfg!(windows) { ".exe" } else { "" };
    format!("{:016x}-{}{ext}", t.hash, t.unpacked_len)
}

#[cfg(test)]
mod tests;
