//! File helpers both runtimes share.

/// `path` as a person reads it: on Windows without the `\\?\` that a full
/// path from the system starts with (`\\?\C:\shop` is `C:\shop`, and
/// `\\?\UNC\server\share` is `\\server\share`).
pub fn shown_path(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    text.strip_prefix(r"\\?\")
        .map_or(text.clone(), str::to_string)
}

/// Writes through a temporary file that is then renamed over `path`, so
/// a crash keeps either the old contents or the new, never half (spec
/// 14.3).
pub fn write_atomically(path: &str, data: &[u8]) -> std::io::Result<()> {
    through_temp(std::path::Path::new(path), data, false)
}

/// The same, with the file forced to the disk before the rename and the
/// folder after it, so a power cut keeps the old or the new too.
pub fn write_durably(path: &str, data: &[u8]) -> std::io::Result<()> {
    let target = std::path::Path::new(path);
    through_temp(target, data, true)?;
    // The rename itself must survive too: force the folder. Windows has
    // no way to open a folder for this; its rename is already in the
    // metadata log.
    #[cfg(unix)]
    if let Some(parent) = target.parent() {
        let dir = if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        };
        std::fs::File::open(dir)?.sync_all()?;
    }
    Ok(())
}

/// `data` into a temporary file beside `target`, renamed over it; forced
/// to the disk first when `durable`. A step that fails removes the
/// temporary file, so a full disk leaves nothing behind. Its name is
/// the program's and a count, so two tasks never share one.
fn through_temp(target: &std::path::Path, data: &[u8], durable: bool) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let name = target
        .file_name()
        .map_or("file".into(), |n| n.to_string_lossy().to_string());
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let temp = target.with_file_name(format!(".{name}.kavach-{}-{n}.tmp", std::process::id()));
    let written = std::fs::File::create(&temp).and_then(|mut file| {
        file.write_all(data)?;
        if durable {
            file.sync_all()?;
        }
        Ok(())
    });
    written
        .and_then(|()| std::fs::rename(&temp, target))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp);
        })
}

/// Where `part` first appears in `data` (`Bytes.index_of`, spec 2.13);
/// an empty `part` is at 0.
pub fn find_bytes(data: &[u8], part: &[u8]) -> Option<usize> {
    if part.is_empty() {
        return Some(0);
    }
    data.windows(part.len()).position(|w| w == part)
}

/// XXH64 with seed 0 (spec 2.13), as its reference implementation
/// computes it: four lanes over 32-byte stripes, then the tail.
pub fn xxhash64(data: &[u8]) -> u64 {
    const P1: u64 = 0x9E37_79B1_85EB_CA87;
    const P2: u64 = 0xC2B2_AE3D_27D4_EB4F;
    const P3: u64 = 0x1656_67B1_9E37_79F9;
    const P4: u64 = 0x85EB_CA77_C2B2_AE63;
    const P5: u64 = 0x27D4_EB2F_1656_67C5;
    let word = |b: &[u8]| u64::from_le_bytes(b[..8].try_into().unwrap_or_default());
    let round = |acc: u64, lane: u64| {
        acc.wrapping_add(lane.wrapping_mul(P2))
            .rotate_left(31)
            .wrapping_mul(P1)
    };
    let merge = |acc: u64, v: u64| (acc ^ round(0, v)).wrapping_mul(P1).wrapping_add(P4);
    let mut rest = data;
    let mut h = if data.len() >= 32 {
        let mut v = [P1.wrapping_add(P2), P2, 0, 0u64.wrapping_sub(P1)];
        while rest.len() >= 32 {
            for (i, lane) in v.iter_mut().enumerate() {
                *lane = round(*lane, word(&rest[8 * i..]));
            }
            rest = &rest[32..];
        }
        let h = v[0]
            .rotate_left(1)
            .wrapping_add(v[1].rotate_left(7))
            .wrapping_add(v[2].rotate_left(12))
            .wrapping_add(v[3].rotate_left(18));
        v.iter().fold(h, |h, lane| merge(h, *lane))
    } else {
        P5
    };
    h = h.wrapping_add(data.len() as u64);
    while rest.len() >= 8 {
        h = (h ^ round(0, word(rest)))
            .rotate_left(27)
            .wrapping_mul(P1)
            .wrapping_add(P4);
        rest = &rest[8..];
    }
    if rest.len() >= 4 {
        let half = u64::from(u32::from_le_bytes(rest[..4].try_into().unwrap_or_default()));
        h = (h ^ half.wrapping_mul(P1))
            .rotate_left(23)
            .wrapping_mul(P2)
            .wrapping_add(P3);
        rest = &rest[4..];
    }
    for b in rest {
        h = (h ^ u64::from(*b).wrapping_mul(P5))
            .rotate_left(11)
            .wrapping_mul(P1);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(P2);
    h ^= h >> 29;
    h = h.wrapping_mul(P3);
    h ^ (h >> 32)
}

/// CRC32C (Castagnoli), the checksum storage engines and gRPC use:
/// slicing-by-8 over tables built once (spec 14.2).
pub fn crc32c(data: &[u8]) -> u32 {
    use std::sync::OnceLock;
    static TABLES: OnceLock<[[u32; 256]; 8]> = OnceLock::new();
    let t = TABLES.get_or_init(|| {
        let mut t = [[0u32; 256]; 8];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0x82F6_3B78 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            t[0][n as usize] = c;
        }
        for k in 1..8 {
            for n in 0..256 {
                let c = t[k - 1][n];
                t[k][n] = t[0][(c & 0xFF) as usize] ^ (c >> 8);
            }
        }
        t
    });
    let mut crc = !0u32;
    let (chunks, rest) = data.as_chunks::<8>();
    for c in chunks {
        let low = crc ^ u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        let high = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
        crc = t[7][(low & 0xFF) as usize]
            ^ t[6][((low >> 8) & 0xFF) as usize]
            ^ t[5][((low >> 16) & 0xFF) as usize]
            ^ t[4][(low >> 24) as usize]
            ^ t[3][(high & 0xFF) as usize]
            ^ t[2][((high >> 8) & 0xFF) as usize]
            ^ t[1][((high >> 16) & 0xFF) as usize]
            ^ t[0][(high >> 24) as usize];
    }
    for &b in rest {
        crc = t[0][((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    /// Paths in messages read as people write them: no `\\?\`.
    #[test]
    fn a_shown_path_has_no_verbatim_prefix() {
        let cases = [
            (r"\\?\C:\shop\agents\Bot", r"C:\shop\agents\Bot"),
            (r"\\?\UNC\server\share\Bot", r"\\server\share\Bot"),
            (r"C:\shop", r"C:\shop"),
            ("agents/Bot", "agents/Bot"),
        ];
        for (path, shown) in cases {
            assert_eq!(
                super::shown_path(std::path::Path::new(path)),
                shown,
                "{path}"
            );
        }
    }

    /// A write that fails leaves no temporary file behind.
    #[test]
    fn a_failed_write_leaves_nothing_behind() {
        let dir = std::env::temp_dir().join(format!("kavach-files-{}", std::process::id()));
        let target = dir.join("taken");
        std::fs::create_dir_all(target.join("inside")).unwrap();
        // A folder that isn't empty can't be renamed over.
        let path = target.to_str().unwrap();
        assert!(super::write_atomically(path, b"x").is_err());
        assert!(super::write_durably(path, b"x").is_err());
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(left.len(), 1, "{left:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn xxhash64_matches_the_reference() {
        // From the reference implementation (Python's `xxhash` 3).
        assert_eq!(super::xxhash64(b""), 0xEF46_DB37_51D8_E999);
        assert_eq!(super::xxhash64(b"a"), 0xD24E_C4F1_A98C_6E5B);
        assert_eq!(super::xxhash64(b"123456789"), 0x8CB8_41DB_40E6_AE83);
        assert_eq!(super::xxhash64(&[0u8; 32]), 0xF6E9_BE5D_7063_2CF5);
        assert_eq!(super::xxhash64(&b"abc".repeat(100)), 0x517D_55D7_8F9D_AC2B);
    }

    #[test]
    fn crc32c_matches_the_known_vectors() {
        // RFC 3720 appendix B.4 and the classic check value.
        assert_eq!(super::crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(super::crc32c(b""), 0);
        assert_eq!(super::crc32c(&[0u8; 32]), 0x8A91_36AA);
        assert_eq!(super::crc32c(&[0xFFu8; 32]), 0x62A8_AB43);
    }
}
