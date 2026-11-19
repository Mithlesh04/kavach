// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! KTX2 files of compressed textures (spec 14.24): blocks a GPU reads as
//! they are, BC on desktops and ASTC on phones, 4 to 8 times smaller in
//! its memory than RGBA. Read here, with no library: its header, then
//! each mip level's place in the file.

/// A KTX2 file's texture: its format, size and mip levels, largest first.
#[derive(Debug, PartialEq)]
pub struct Ktx2<'a> {
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub levels: Vec<&'a [u8]>,
}

/// The file's first 12 bytes.
const IDENTIFIER: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

/// The texture format of Vulkan's format number `vk`, for the block
/// formats Kavach reads.
fn format_of(vk: u32) -> Option<wgpu::TextureFormat> {
    use wgpu::{AstcBlock, AstcChannel, TextureFormat as F};
    Some(match vk {
        131 | 133 => F::Bc1RgbaUnorm,
        132 | 134 => F::Bc1RgbaUnormSrgb,
        135 => F::Bc2RgbaUnorm,
        136 => F::Bc2RgbaUnormSrgb,
        137 => F::Bc3RgbaUnorm,
        138 => F::Bc3RgbaUnormSrgb,
        139 => F::Bc4RUnorm,
        140 => F::Bc4RSnorm,
        141 => F::Bc5RgUnorm,
        142 => F::Bc5RgSnorm,
        143 => F::Bc6hRgbUfloat,
        144 => F::Bc6hRgbFloat,
        145 => F::Bc7RgbaUnorm,
        146 => F::Bc7RgbaUnormSrgb,
        157 => F::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::Unorm,
        },
        158 => F::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::UnormSrgb,
        },
        _ => return None,
    })
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The texture in KTX2 file `bytes`, or why it can't be read.
pub fn read(bytes: &[u8]) -> Result<Ktx2<'_>, String> {
    if !bytes.starts_with(&IDENTIFIER) {
        return Err("this is not a KTX2 file".into());
    }
    let cut = || "this KTX2 file is cut short".to_string();
    let field = |at: usize| u32_at(bytes, at).ok_or_else(cut);
    let vk = field(12)?;
    let format = format_of(vk).ok_or_else(|| {
        format!("format {vk} isn't one Kavach reads: BC1 to BC7 and ASTC 4×4 are")
    })?;
    let (width, height, depth) = (field(20)?, field(24)?, field(28)?);
    let (layers, faces, levels) = (field(32)?, field(36)?, field(40)?.max(1));
    if field(44)? != 0 {
        return Err(
            "supercompressed KTX2 files (Basis Universal, Zstandard) aren't read yet".into(),
        );
    }
    if depth > 1 || layers > 1 || faces != 1 {
        return Err("only flat textures are read: no depth, layers or cube faces".into());
    }
    if !(1..=crate::texture::MOST).contains(&width) || !(1..=crate::texture::MOST).contains(&height)
    {
        return Err(format!(
            "a texture is 1 to {} pixels wide and high, not {width} × {height}",
            crate::texture::MOST
        ));
    }
    if width % 4 != 0 || height % 4 != 0 {
        return Err(format!(
            "a compressed texture's width and height are each a multiple of 4, not {width} × {height}"
        ));
    }
    let block = format.block_copy_size(None).unwrap_or(16) as u64;
    let mut out = Vec::new();
    for k in 0..levels {
        let at = 80 + 24 * k as usize;
        let (offset, length) = (
            u64_at(bytes, at).ok_or_else(cut)?,
            u64_at(bytes, at + 8).ok_or_else(cut)?,
        );
        let side = |v: u32| u64::from((v >> k).max(1).div_ceil(4));
        if length != side(width) * side(height) * block {
            return Err(format!(
                "mip level {k} holds {length} bytes, not the size its blocks take"
            ));
        }
        let level = usize::try_from(offset)
            .ok()
            .and_then(|o| bytes.get(o..o.checked_add(length as usize)?))
            .ok_or_else(cut)?;
        out.push(level);
    }
    Ok(Ktx2 {
        format,
        width,
        height,
        levels: out,
    })
}

/// A KTX2 file of `levels` of Vulkan format `vk`, `width` × `height`,
/// with nothing else in it: for tests, here and in the backends.
pub fn file(vk: u32, width: u32, height: u32, levels: &[Vec<u8>]) -> Vec<u8> {
    let mut out = IDENTIFIER.to_vec();
    for v in [vk, 1, width, height, 0, 0, 1, levels.len() as u32, 0] {
        out.extend(v.to_le_bytes());
    }
    out.extend([0u8; 32]); // no data format, key-values or global data
    let mut offset = 80 + 24 * levels.len() as u64;
    for l in levels {
        out.extend(offset.to_le_bytes());
        out.extend((l.len() as u64).to_le_bytes());
        out.extend((l.len() as u64).to_le_bytes());
        offset += l.len() as u64;
    }
    for l in levels {
        out.extend(l);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bc1_file_with_mip_levels_is_read() {
        // 8 × 8: four blocks of 8 bytes, then one block for 4 × 4.
        let f = file(131, 8, 8, &[vec![1; 32], vec![2; 8]]);
        let t = read(&f).unwrap();
        assert_eq!(
            (t.format, t.width, t.height),
            (wgpu::TextureFormat::Bc1RgbaUnorm, 8, 8)
        );
        assert_eq!(t.levels, vec![&[1u8; 32][..], &[2u8; 8][..]]);
    }

    #[test]
    fn what_kavach_cant_read_is_refused() {
        assert!(read(b"not a texture").unwrap_err().contains("not a KTX2"));
        assert!(
            read(&file(37, 4, 4, &[vec![0; 64]]))
                .unwrap_err()
                .contains("isn't one Kavach reads")
        );
        assert!(
            read(&file(131, 6, 4, &[vec![0; 16]]))
                .unwrap_err()
                .contains("multiple of 4")
        );
        assert!(
            read(&file(145, 4, 4, &[vec![0; 8]]))
                .unwrap_err()
                .contains("not the size")
        );
        let cut = file(145, 4, 4, &[vec![0; 16]]);
        assert!(
            read(&cut[..cut.len() - 1])
                .unwrap_err()
                .contains("cut short")
        );
    }
}
