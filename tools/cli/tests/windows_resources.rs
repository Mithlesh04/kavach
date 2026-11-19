// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `kavach build` gives a Windows executable a manifest, and the icon
//! `[build] icon` names (spec 10.5, decision 0120); an icon that isn't a
//! PNG is an error on every system. `[build] console = false` makes a
//! window program.

use std::path::Path;
use std::process::Command;

/// A PNG of 32 × 32 orange pixels, as `[build] icon` takes.
fn png() -> Vec<u8> {
    fn chunk(kind: &[u8], data: &[u8]) -> Vec<u8> {
        let mut c = (data.len() as u32).to_be_bytes().to_vec();
        c.extend_from_slice(kind);
        c.extend_from_slice(data);
        let mut crc = !0u32;
        for b in kind.iter().chain(data) {
            crc ^= u32::from(*b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        c.extend_from_slice(&(!crc).to_be_bytes());
        c
    }
    let mut header = 32u32.to_be_bytes().to_vec();
    header.extend_from_slice(&32u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    // Stored deflate blocks: each row a filter byte and 32 pixels.
    let row: Vec<u8> = std::iter::once(0)
        .chain([255, 128, 0, 255].repeat(32))
        .collect();
    let raw = row.repeat(32);
    let mut z = vec![0x78, 0x01, 1];
    z.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    z.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    z.extend_from_slice(&raw);
    let (mut a, mut b) = (1u32, 0u32);
    for x in &raw {
        a = (a + u32::from(*x)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend(chunk(b"IHDR", &header));
    png.extend(chunk(b"IDAT", &z));
    png.extend(chunk(b"IEND", &[]));
    png
}

fn build(dir: &Path, icon: &[u8]) -> std::process::Output {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("icon.png"), icon).unwrap();
    std::fs::write(
        dir.join("kavach.toml"),
        "[package]\nname = \"iconapp\"\n\n[build]\nicon = \"icon.png\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("main.kav"), "fn main() {\n    print(\"hi\")\n}\n").unwrap();
    Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", "main.kav"])
        .current_dir(dir)
        .output()
        .unwrap()
}

/// The resource types at the top of a PE file's resource directory.
fn resource_types(exe: &[u8]) -> Vec<u32> {
    let u16_at = |at: usize| u16::from_le_bytes([exe[at], exe[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(exe[at..at + 4].try_into().unwrap()) as usize;
    let pe = u32_at(0x3C);
    let sections = u16_at(pe + 6);
    let optional = pe + 24;
    // PE32+: the data directories start 112 bytes in; resources are third.
    let rva = u32_at(optional + 112 + 2 * 8);
    let table = optional + u16_at(pe + 20);
    let offset = (0..sections)
        .map(|i| table + i * 40)
        .find_map(|s| {
            let (start, size, raw) = (u32_at(s + 12), u32_at(s + 8), u32_at(s + 20));
            (start..start + size)
                .contains(&rva)
                .then(|| rva - start + raw)
        })
        .expect("a resource section");
    let entries = u16_at(offset + 12) + u16_at(offset + 14);
    (0..entries)
        .map(|i| u32_at(offset + 16 + i * 8) as u32)
        .collect()
}

#[test]
fn a_built_program_has_its_icon_and_a_manifest() {
    let dir = std::env::temp_dir().join(format!("kavach-icon-{}", std::process::id()));
    let out = build(&dir, &png());
    let exe = std::fs::read(dir.join(format!("main{}", std::env::consts::EXE_SUFFIX)));
    let not_png = build(&dir.join("bad"), b"GIF89a, not a PNG at all");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    if cfg!(windows) {
        // An icon (3), its group (14) and the manifest (24).
        assert_eq!(resource_types(&exe.unwrap()), vec![3, 14, 24]);
    }
    assert!(!not_png.status.success());
    let err = String::from_utf8_lossy(&not_png.stderr);
    assert!(err.contains("the icon must be a PNG file"), "{err}");
}

/// The subsystem a PE file asks for: 2 a window program, 3 a console one.
fn subsystem(exe: &[u8]) -> u16 {
    let pe = u32::from_le_bytes(exe[0x3C..0x40].try_into().unwrap()) as usize;
    let at = pe + 24 + 68;
    u16::from_le_bytes([exe[at], exe[at + 1]])
}

#[test]
fn console_false_makes_a_window_program() {
    let dir = std::env::temp_dir().join(format!("kavach-console-{}", std::process::id()));
    let made = |sub: &str, build: &str| {
        let d = dir.join(sub);
        std::fs::create_dir_all(&d).unwrap();
        let toml = format!(
            "[package]
name = \"{sub}\"
{build}"
        );
        std::fs::write(d.join("kavach.toml"), toml).unwrap();
        std::fs::write(
            d.join("main.kav"),
            "fn main() {
    print(\"hi\")
}
",
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .args(["build", "main.kav"])
            .current_dir(&d)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        d.join(format!("main{}", std::env::consts::EXE_SUFFIX))
    };
    let window = made(
        "window",
        "
[build]
console = false
",
    );
    let console = made("console", "");
    let (window_exe, console_exe) = (std::fs::read(&window), std::fs::read(&console));
    // A window program still runs, and what it prints reaches a pipe.
    let ran = Command::new(&window).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(ran.status.success());
    if cfg!(windows) {
        assert_eq!(subsystem(&window_exe.unwrap()), 2);
        assert_eq!(subsystem(&console_exe.unwrap()), 3);
        assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "hi");
    }
}
