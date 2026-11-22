// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
    sweep_once(target);
    // Another program's sweep can take the temporary file in the moment
    // before it is locked or after it is let go: the rename then finds
    // nothing, and the write starts again with a new one.
    for _ in 0..4 {
        if write_and_rename(target, data, durable)? {
            return Ok(());
        }
    }
    Err(std::io::Error::other("its temporary file was removed"))
}

/// One try of `through_temp`: `false` when the temporary file was gone
/// at the rename.
fn write_and_rename(target: &std::path::Path, data: &[u8], durable: bool) -> std::io::Result<bool> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let name = target
        .file_name()
        .map_or("file".into(), |n| n.to_string_lossy().to_string());
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let temp = target.with_file_name(format!(".{name}.kavach-{}-{n}.tmp", std::process::id()));
    std::fs::File::create(&temp)
        .and_then(|mut file| {
            // Held until the file is complete, so a sweep leaves it alone.
            file.lock()?;
            file.write_all(data)?;
            if durable {
                file.sync_all()?;
            }
            Ok(())
        })
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp);
        })?;
    match std::fs::rename(&temp, target) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !temp.exists() => Ok(false),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e)
        }
    }
}

/// Removes, at the first whole-file write into a folder in a run, the
/// temporary files that writes cut short left there (spec 14.3): those
/// of other programs that nobody holds locked. This program's own are
/// writes in progress. A failure only leaves a file where it was.
fn sweep_once(target: &std::path::Path) {
    use std::collections::HashSet;
    use std::sync::Mutex;
    static SWEPT: Mutex<Option<HashSet<std::path::PathBuf>>> = Mutex::new(None);
    let dir = match target.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => std::path::Path::new("."),
    };
    let first = SWEPT
        .lock()
        .is_ok_and(|mut swept| swept.get_or_insert_default().insert(dir.to_path_buf()));
    if !first {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let me = std::process::id().to_string();
    for entry in entries.flatten() {
        let name = entry.file_name();
        match name.to_str().and_then(temp_owner) {
            Some(owner) if owner != me => {}
            _ => continue,
        }
        let path = entry.path();
        if let Ok(file) = std::fs::File::open(&path)
            && file.try_lock().is_ok()
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// The process number in a temporary file's name,
/// `.name.kavach-<pid>-<n>.tmp`; `None` for any other name.
fn temp_owner(name: &str) -> Option<&str> {
    let middle = name.strip_prefix('.')?.strip_suffix(".tmp")?;
    let (target, tail) = middle.rsplit_once(".kavach-")?;
    let (pid, n) = tail.split_once('-')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (!target.is_empty() && digits(pid) && digits(n)).then_some(pid)
}

/// Where `part` first appears in `data` (`Bytes.index_of`, spec 2.13);
/// an empty `part` is at 0.
pub fn find_bytes(data: &[u8], part: &[u8]) -> Option<usize> {
    match part {
        [] => Some(0),
        [one] => memchr::memchr(*one, data),
        _ => memchr::memmem::find(data, part),
    }
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

/// The `FsError` variant a failed file operation on `path` is (spec
/// 14.3): `NotFound`, `Denied`, `IsFolder`, `NotEmpty`, `NotText`, or
/// `Failed` with the reason.
pub fn error_variant(path: &str, e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind as K;
    match e.kind() {
        K::NotFound => "NotFound",
        K::IsADirectory => "IsFolder",
        K::DirectoryNotEmpty => "NotEmpty",
        K::InvalidData => "NotText",
        // Windows opens a folder as a file with "access denied".
        K::PermissionDenied if std::path::Path::new(path).is_dir() => "IsFolder",
        K::PermissionDenied => "Denied",
        _ => "Failed",
    }
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
    fn temporary_names_give_their_program() {
        let cases = [
            (".a.txt.kavach-12-0.tmp", Some("12")),
            (".a.kavach-b.kavach-7-31.tmp", Some("7")),
            (".a.txt.tmp", None),
            ("a.txt.kavach-12-0.tmp", None),
            (".a.txt.kavach-12-0.tmp.bak", None),
            (".a.txt.kavach-12.tmp", None),
            (".a.txt.kavach--0.tmp", None),
            (".a.txt.kavach-1x-0.tmp", None),
            ("..kavach-12-0.tmp", None),
        ];
        for (name, owner) in cases {
            assert_eq!(super::temp_owner(name), owner, "{name}");
        }
    }

    /// The first write into a folder removes what writes cut short left
    /// there, and nothing else: not a file a write in progress holds, not
    /// this program's own, not a name of another form.
    #[test]
    fn the_first_write_into_a_folder_removes_writes_cut_short() {
        let dir = std::env::temp_dir().join(format!("kavach-sweep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mine = format!(".a.txt.kavach-{}-9.tmp", std::process::id());
        let kept = [".a.txt.tmp", "b.kavach-0-0.tmp", ".b.kavach-1-0.tmp", &mine];
        for name in kept.iter().chain(&[".a.txt.kavach-0-0.tmp"]) {
            std::fs::write(dir.join(name), b"half").unwrap();
        }
        // A write in progress in another program: its file is locked.
        let held = std::fs::File::open(dir.join(".b.kavach-1-0.tmp")).unwrap();
        held.lock().unwrap();
        super::write_atomically(dir.join("a.txt").to_str().unwrap(), b"whole").unwrap();
        drop(held);
        let mut left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        let mut want: Vec<_> = kept.iter().map(|s| s.to_string()).collect();
        want.push("a.txt".into());
        want.sort();
        assert_eq!(left, want);
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"whole");
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
