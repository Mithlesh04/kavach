// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The fonts installed on the machine (spec 14.31): the files in the
//! system's font folders, each one's family and style read from its
//! `name` table, and a fallback for text a font lacks, found from their
//! `cmap` tables. Only those two tables are read from each file, so
//! looking through hundreds of fonts takes milliseconds. Shaping stays
//! Kavach's own (decision 0121): the system only says where fonts are.

use rustybuzz::ttf_parser::{cmap, name, name_id};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// A font file installed on the machine: the first font in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub family: String,
    pub style: String,
    pub path: PathBuf,
}

/// The folders fonts are installed in, on this system.
fn folders() -> Vec<PathBuf> {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut out = Vec::new();
    if cfg!(windows) {
        out.extend(env("WINDIR").map(|w| w.join("Fonts")));
        out.extend(env("LOCALAPPDATA").map(|l| l.join("Microsoft/Windows/Fonts")));
    } else if cfg!(target_os = "macos") {
        out.extend(["/System/Library/Fonts", "/Library/Fonts"].map(PathBuf::from));
        out.extend(env("HOME").map(|h| h.join("Library/Fonts")));
    } else {
        out.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        out.extend(env("HOME").map(|h| h.join(".local/share/fonts")));
        out.extend(env("HOME").map(|h| h.join(".fonts")));
    }
    out
}

/// Families tried first for a fallback, as each system's own text does.
fn preferred() -> &'static [&'static str] {
    if cfg!(windows) {
        &[
            "Segoe UI",
            "Segoe UI Symbol",
            "Segoe UI Emoji",
            "Nirmala UI",
            "Microsoft YaHei",
            "Yu Gothic",
            "Malgun Gothic",
            "Microsoft JhengHei",
            "Ebrima",
            "Leelawadee UI",
            "Segoe UI Historic",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "Helvetica Neue",
            "Apple Color Emoji",
            "PingFang SC",
            "Hiragino Sans",
            "Apple SD Gothic Neo",
            "Kohinoor Devanagari",
        ]
    } else {
        &[
            "Noto Sans",
            "DejaVu Sans",
            "Noto Color Emoji",
            "Noto Sans CJK SC",
            "Noto Sans Devanagari",
            "Noto Sans Arabic",
        ]
    }
}

/// The font files under `dir` and its folders, at most 6 deep.
fn files(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut all: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    all.sort();
    for p in all {
        let ext = (p.extension().and_then(|e| e.to_str()))
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if p.is_dir() && depth < 6 {
            files(&p, depth + 1, out);
        } else if matches!(ext.as_str(), "ttf" | "otf" | "ttc") {
            out.push(p);
        }
    }
}

/// Table `tag` of the first font in `file`, read alone.
fn table(file: &mut File, tag: &[u8; 4]) -> Option<Vec<u8>> {
    let mut head = [0u8; 12];
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_exact(&mut head).ok()?;
    let mut start = 0u64;
    if &head[..4] == b"ttcf" {
        let mut first = [0u8; 4];
        file.read_exact(&mut first).ok()?;
        start = u64::from(u32::from_be_bytes(first));
        file.seek(SeekFrom::Start(start)).ok()?;
        file.read_exact(&mut head).ok()?;
    }
    let count = u16::from_be_bytes([head[4], head[5]]);
    let mut records = vec![0u8; 16 * usize::from(count)];
    file.seek(SeekFrom::Start(start + 12)).ok()?;
    file.read_exact(&mut records).ok()?;
    let record = records
        .as_chunks::<16>()
        .0
        .iter()
        .find(|r| &r[..4] == tag)?;
    let word =
        |k: usize| u32::from_be_bytes([record[k], record[k + 1], record[k + 2], record[k + 3]]);
    let (offset, length) = (word(8), word(12));
    // Tables of more than 16 MB are not what a font has.
    if length > 1 << 24 {
        return None;
    }
    let mut data = vec![0u8; length as usize];
    file.seek(SeekFrom::Start(u64::from(offset))).ok()?;
    file.read_exact(&mut data).ok()?;
    Some(data)
}

/// The family and style a `name` table gives, in English where it can.
pub(crate) fn names(data: &[u8]) -> Option<(String, String)> {
    let table = name::Table::parse(data)?;
    let find = |ids: &[u16]| {
        ids.iter().find_map(|&id| {
            let all = table.names.into_iter().filter(|n| n.name_id == id);
            let english = all.clone().find(|n| n.language_id == 0x409);
            english.or_else(|| all.clone().next())?.to_string()
        })
    };
    let family = find(&[name_id::TYPOGRAPHIC_FAMILY, name_id::FAMILY])?;
    let style = find(&[name_id::TYPOGRAPHIC_SUBFAMILY, name_id::SUBFAMILY]).unwrap_or_default();
    Some((family, style))
}

/// The fonts in `folders`, by family and style.
pub(crate) fn scan(folders: &[PathBuf]) -> Vec<Installed> {
    let mut paths = Vec::new();
    for f in folders {
        files(f, 0, &mut paths);
    }
    let mut out: Vec<Installed> = paths
        .into_iter()
        .filter_map(|path| {
            let data = table(&mut File::open(&path).ok()?, b"name")?;
            let (family, style) = names(&data)?;
            Some(Installed {
                family,
                style,
                path,
            })
        })
        .collect();
    out.sort_by(|a, b| (&a.family, &a.style, &a.path).cmp(&(&b.family, &b.style, &b.path)));
    out
}

/// The fonts installed on this machine, looked for once.
pub fn installed() -> &'static [Installed] {
    static ALL: OnceLock<Vec<Installed>> = OnceLock::new();
    ALL.get_or_init(|| scan(&folders()))
}

/// Whether the font at `path` has a glyph for every one of `chars`.
fn covers(path: &Path, chars: &[char]) -> bool {
    let Some(data) = File::open(path)
        .ok()
        .and_then(|mut f| table(&mut f, b"cmap"))
    else {
        return false;
    };
    let Some(table) = cmap::Table::parse(&data) else {
        return false;
    };
    chars.iter().all(|&c| {
        (table.subtables.into_iter())
            .filter(cmap::Subtable::is_unicode)
            .any(|s| s.glyph_index(u32::from(c)).is_some_and(|g| g.0 != 0))
    })
}

/// Of `fonts`, the first with a glyph for each character of `text` but
/// spaces and controls: the families of `first` before the others, and
/// regular styles first. `None` when no font has them all.
pub(crate) fn fallback_in(fonts: &[Installed], first: &[&str], text: &str) -> Option<usize> {
    let mut chars: Vec<char> = text
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();
    chars.sort_unstable();
    chars.dedup();
    let rank = |f: &Installed| {
        let place = first
            .iter()
            .position(|p| *p == f.family)
            .unwrap_or(first.len());
        let plain = !matches!(f.style.as_str(), "Regular" | "Normal" | "Book" | "");
        (place, plain)
    };
    let mut order: Vec<usize> = (0..fonts.len()).collect();
    order.sort_by_key(|&k| rank(&fonts[k]));
    order.into_iter().find(|&k| covers(&fonts[k].path, &chars))
}

/// An installed font with a glyph for each character of `text`, found
/// once for each text: its place in `installed()`.
pub fn fallback(text: &str) -> Option<usize> {
    static FOUND: OnceLock<Mutex<HashMap<String, Option<usize>>>> = OnceLock::new();
    let found = FOUND.get_or_init(Mutex::default);
    if let Some(&k) = found.lock().unwrap_or_else(|e| e.into_inner()).get(text) {
        return k;
    }
    let k = fallback_in(installed(), preferred(), text);
    let mut all = found.lock().unwrap_or_else(|e| e.into_inner());
    all.insert(text.to_string(), k);
    k
}

/// `installed()` as `std.font` reads it: three texts a font, its family,
/// style and path.
pub fn installed_flat() -> Vec<String> {
    (installed().iter())
        .flat_map(|f| {
            [
                f.family.clone(),
                f.style.clone(),
                f.path.display().to_string(),
            ]
        })
        .collect()
}

/// The bytes of the installed font at `path`; nothing for a path that
/// isn't an installed font's, so this reads no other file.
pub fn read_installed(path: &str) -> Vec<u8> {
    let known = installed().iter().any(|f| f.path.as_os_str() == path);
    match known {
        true => std::fs::read(path).unwrap_or_default(),
        false => Vec::new(),
    }
}
