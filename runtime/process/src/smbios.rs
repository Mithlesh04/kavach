// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The SMBIOS tables (DMTF DSP0134), the firmware's own inventory: memory
//! modules and the machine's ids (spec 14.25, decision 0112). Windows
//! hands them to any program (`GetSystemFirmwareTable`); Linux only to
//! root (`/sys/firmware/dmi/tables/DMI`). Plain bytes in, text out.

/// One structure: its type, its formatted bytes (from the type on), and
/// its strings, numbered from 1.
struct Structure<'a> {
    kind: u8,
    bytes: &'a [u8],
    strings: Vec<String>,
}

impl Structure<'_> {
    fn byte(&self, at: usize) -> Option<u8> {
        self.bytes.get(at).copied()
    }

    fn word(&self, at: usize) -> Option<u16> {
        Some(u16::from_le_bytes([self.byte(at)?, self.byte(at + 1)?]))
    }

    fn dword(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(
            self.bytes.get(at..at + 4)?.try_into().ok()?,
        ))
    }

    /// The string whose number is at `at`; `""` for none.
    fn text(&self, at: usize) -> String {
        match self.byte(at) {
            Some(n @ 1..) => self
                .strings
                .get(n as usize - 1)
                .cloned()
                .unwrap_or_default(),
            _ => String::new(),
        }
    }
}

/// Every structure of `table`, in order, up to the end marker (type 127).
fn structures(table: &[u8]) -> Vec<Structure<'_>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 4 <= table.len() {
        let (kind, len) = (table[at], table[at + 1] as usize);
        if len < 4 || at + len > table.len() {
            break;
        }
        let bytes = &table[at..at + len];
        // The strings follow, each ending with 0, the last with two.
        let mut end = at + len;
        let mut strings = Vec::new();
        loop {
            let start = end;
            while end < table.len() && table[end] != 0 {
                end += 1;
            }
            if end == start {
                end += 1;
                break;
            }
            strings.push(
                String::from_utf8_lossy(&table[start..end])
                    .trim()
                    .to_string(),
            );
            end += 1;
        }
        if strings.is_empty() {
            end += 1;
        }
        out.push(Structure {
            kind,
            bytes,
            strings,
        });
        if kind == 127 {
            break;
        }
        at = end;
    }
    out
}

/// The name of a memory type (SMBIOS type 17, offset 0x12).
fn memory_kind(code: u8) -> &'static str {
    match code {
        0x12 => "DDR",
        0x13 => "DDR2",
        0x18 => "DDR3",
        0x1A => "DDR4",
        0x1B => "LPDDR",
        0x1C => "LPDDR2",
        0x1D => "LPDDR3",
        0x1E => "LPDDR4",
        0x22 => "DDR5",
        0x23 => "LPDDR5",
        _ => "",
    }
}

/// Each memory slot: its name, size in bytes (0 when empty), kind, speed
/// in MHz (MT/s), maker and part number.
pub fn memory_modules(table: &[u8]) -> Vec<Vec<String>> {
    let mb = 1024 * 1024u64;
    (structures(table).into_iter())
        .filter(|s| s.kind == 17 && s.bytes.len() >= 0x15)
        .map(|s| {
            let size = match s.word(0x0C).unwrap_or(0) {
                0 | 0xFFFF => 0,
                0x7FFF => s.dword(0x1C).unwrap_or(0) as u64 * mb,
                n if n & 0x8000 != 0 => (n & 0x7FFF) as u64 * 1024,
                n => n as u64 * mb,
            };
            vec![
                s.text(0x10),
                size.to_string(),
                memory_kind(s.byte(0x12).unwrap_or(0)).to_string(),
                s.word(0x15).unwrap_or(0).to_string(),
                if size == 0 {
                    String::new()
                } else {
                    s.text(0x17)
                },
                if size == 0 {
                    String::new()
                } else {
                    s.text(0x1A)
                },
            ]
        })
        .collect()
}

/// The system's UUID (type 1) and the board's serial number (type 2), as
/// text; `""` where the firmware gives none.
pub fn ids(table: &[u8]) -> (String, String) {
    let all = structures(table);
    let uuid = all
        .iter()
        .find(|s| s.kind == 1 && s.bytes.len() >= 0x18)
        .map(|s| {
            let u = &s.bytes[0x08..0x18];
            if u.iter().all(|b| *b == 0) || u.iter().all(|b| *b == 0xFF) {
                return String::new();
            }
            // The first three fields are little-endian (SMBIOS 2.6).
            format!(
                "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                u[3], u[2], u[1], u[0], u[5], u[4], u[7], u[6], u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
            )
        })
        .unwrap_or_default();
    let serial = (all.iter().find(|s| s.kind == 2).map(|s| s.text(0x07))).unwrap_or_default();
    (uuid, serial)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table with a filled slot, an empty one, a board and the end.
    fn table() -> Vec<u8> {
        let mut t = Vec::new();
        let mut module = vec![0u8; 0x22];
        module[0] = 17;
        module[1] = 0x22;
        module[0x0C..0x0E].copy_from_slice(&16384u16.to_le_bytes());
        module[0x10] = 1;
        module[0x12] = 0x22;
        module[0x15..0x17].copy_from_slice(&4800u16.to_le_bytes());
        module[0x17] = 2;
        module[0x1A] = 3;
        t.extend(&module);
        t.extend(b"DIMM A\0Maker\0PART-1\0\0");
        let mut empty = vec![0u8; 0x22];
        empty[0] = 17;
        empty[1] = 0x22;
        empty[0x10] = 1;
        t.extend(&empty);
        t.extend(b"DIMM B\0\0");
        let mut board = vec![0u8; 0x08];
        board[0] = 2;
        board[1] = 0x08;
        board[0x07] = 1;
        t.extend(&board);
        t.extend(b"SERIAL-9\0\0");
        t.extend([127, 4, 0, 0, 0, 0]);
        t
    }

    #[test]
    fn modules_slots_and_ids_are_read() {
        let modules = memory_modules(&table());
        assert_eq!(modules.len(), 2);
        assert_eq!(
            modules[0],
            ["DIMM A", "17179869184", "DDR5", "4800", "Maker", "PART-1"]
        );
        assert_eq!(modules[1], ["DIMM B", "0", "", "0", "", ""]);
        assert_eq!(ids(&table()), (String::new(), "SERIAL-9".to_string()));
        assert!(memory_modules(&[1, 2, 3]).is_empty());
    }
}
