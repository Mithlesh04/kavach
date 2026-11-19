// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The resources of a Windows executable (spec 10.5, decision 0120): a
//! manifest always, and the program's icon when `kavach.toml` names one,
//! written as a `.res` file for `link.exe`, with no Windows SDK.

/// The resource types this writes (`RT_ICON`, `RT_GROUP_ICON`,
/// `RT_MANIFEST`).
const ICON: u16 = 3;
const GROUP_ICON: u16 = 14;
const MANIFEST: u16 = 24;

/// US English, the language the resources are filed under.
const LANGUAGE: u16 = 0x0409;

/// Runs as the user who starts it, sharp on high-DPI screens, with long
/// paths, on Windows 10 and later.
const MANIFEST_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"#;

/// The width and height of the PNG `bytes`, if it is one an icon may be:
/// at most 256 × 256.
pub fn check_icon(bytes: &[u8]) -> Result<(u32, u32), String> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err("the icon must be a PNG file".into());
    }
    let word =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let (width, height) = (word(16), word(20));
    if width == 0 || height == 0 || width > 256 || height > 256 {
        return Err(format!(
            "the icon is {width} × {height} pixels; it may be at most 256 × 256"
        ));
    }
    Ok((width, height))
}

/// The `.res` file: the manifest, and the icon `icon` (a PNG that
/// `check_icon` passed) when there is one.
pub fn resource_file(icon: Option<&[u8]>) -> Vec<u8> {
    let mut out = Vec::new();
    // A `.res` file starts with an empty resource.
    entry(&mut out, 0, 0, 0, 0, &[]);
    if let Some(png) = icon {
        let (width, height) = check_icon(png).unwrap_or((256, 256));
        entry(&mut out, ICON, 1, 0x1010, LANGUAGE, png);
        // The group: one icon, number 1 (0 stands for 256 pixels).
        let side = |n: u32| if n >= 256 { 0 } else { n as u8 };
        let mut group = Vec::new();
        for v in [0u16, 1, 1] {
            group.extend_from_slice(&v.to_le_bytes());
        }
        group.extend_from_slice(&[side(width), side(height), 0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(png.len() as u32).to_le_bytes());
        group.extend_from_slice(&1u16.to_le_bytes());
        entry(&mut out, GROUP_ICON, 1, 0x1030, LANGUAGE, &group);
    }
    entry(
        &mut out,
        MANIFEST,
        1,
        0x0030,
        LANGUAGE,
        MANIFEST_XML.as_bytes(),
    );
    out
}

/// One resource: its 32-byte header (numbered type and name), then its
/// data, padded to four bytes.
fn entry(out: &mut Vec<u8>, kind: u16, name: u16, flags: u16, language: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    for v in [0xFFFF, kind, 0xFFFF, name] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&language.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(data);
    out.resize(out.len().next_multiple_of(4), 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The header of a PNG of `w` × `h` pixels, as much as `check_icon`
    /// reads.
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&w.to_be_bytes());
        b.extend_from_slice(&h.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0]);
        b
    }

    #[test]
    fn an_icon_is_a_png_of_at_most_256_pixels() {
        assert_eq!(check_icon(&png(64, 48)), Ok((64, 48)));
        assert!(
            check_icon(&png(257, 10))
                .unwrap_err()
                .contains("at most 256")
        );
        assert!(
            check_icon(b"GIF89a, not a png at all...")
                .unwrap_err()
                .contains("PNG")
        );
    }

    #[test]
    fn a_resource_file_holds_the_icon_its_group_and_the_manifest() {
        let icon = png(256, 256);
        let res = resource_file(Some(&icon));
        // The empty first resource, then each one's size and type.
        assert_eq!(&res[..8], &[0, 0, 0, 0, 32, 0, 0, 0]);
        assert_eq!(res.len() % 4, 0);
        let mut at = 32;
        let mut kinds = Vec::new();
        while at < res.len() {
            let size = u32::from_le_bytes(res[at..at + 4].try_into().unwrap()) as usize;
            kinds.push(u16::from_le_bytes([res[at + 10], res[at + 11]]));
            if kinds.last() == Some(&GROUP_ICON) {
                let group = &res[at + 32..at + 32 + size];
                assert_eq!(&group[..6], &[0, 0, 1, 0, 1, 0]);
                assert_eq!(group[6], 0, "256 pixels is written 0");
                assert_eq!(
                    u32::from_le_bytes(group[14..18].try_into().unwrap()) as usize,
                    icon.len()
                );
            }
            at += (32 + size).next_multiple_of(4);
        }
        assert_eq!(kinds, vec![ICON, GROUP_ICON, MANIFEST]);
        assert_eq!(resource_file(None).len() % 4, 0);
    }
}
