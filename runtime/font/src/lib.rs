// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Fonts for `std.font` (spec 14.31, decision 0121): OpenType and
//! TrueType files read through `ttf-parser`, text shaped through
//! `rustybuzz`, and glyphs drawn as distances into an atlas by our own
//! code (`sdf.rs`). A font is its file's bytes, read again at each call,
//! which `ttf-parser` does without copying. An atlas is kept by number
//! until `std.font` has read its boxes and pixels.

use rustybuzz::ttf_parser::GlyphId;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(test)]
mod sample;
mod sdf;
#[cfg(test)]
mod tests;

/// The largest width or height of an atlas, in pixels.
pub const MAX_ATLAS: u32 = 4096;

fn face(bytes: &[u8]) -> Result<rustybuzz::Face<'_>, String> {
    rustybuzz::Face::from_slice(bytes, 0)
        .ok_or_else(|| "this is not an OpenType or TrueType font, or it is damaged".into())
}

/// The font's ascent, descent and line gap, in ems; or why the bytes
/// aren't a font.
pub fn metrics(bytes: &[u8]) -> Result<[f64; 3], String> {
    let f = face(bytes)?;
    let em = f64::from(f.units_per_em().max(1));
    Ok([f.ascender(), f.descender(), f.line_gap()].map(|v| f64::from(v) / em))
}

/// One shaped glyph: its number, and where its origin goes in pixels from
/// the start of the line, on the baseline, `y` growing downwards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub id: u16,
    pub x: f64,
    pub y: f64,
}

/// `text` shaped at `size` pixels to the em: its glyphs in the order they
/// are drawn, left to right, and the line's width. Nothing for bytes that
/// aren't a font (`std.font` checked them when it loaded the font).
pub fn shape(bytes: &[u8], text: &str, size: f64) -> (Vec<Placed>, f64) {
    let Ok(f) = face(bytes) else {
        return (Vec::new(), 0.0);
    };
    let scale = size / f64::from(f.units_per_em().max(1));
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let shaped = rustybuzz::shape(&f, &[], buffer);
    let mut pen = (0.0, 0.0);
    let mut out = Vec::with_capacity(shaped.len());
    for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        out.push(Placed {
            id: info.glyph_id as u16,
            x: pen.0 + f64::from(pos.x_offset) * scale,
            y: pen.1 - f64::from(pos.y_offset) * scale,
        });
        pen.0 += f64::from(pos.x_advance) * scale;
        pen.1 -= f64::from(pos.y_advance) * scale;
    }
    (out, pen.0)
}

/// `shape`'s result as `std.font` reads it: three numbers a glyph (its
/// number, `x`, `y`), then the width.
pub fn shape_flat(bytes: &[u8], text: &str, size: f64) -> Vec<f64> {
    let (glyphs, width) = shape(bytes, text, size);
    let mut out: Vec<f64> = glyphs
        .iter()
        .flat_map(|g| [f64::from(g.id), g.x, g.y])
        .collect();
    out.push(width);
    out
}

/// Where a glyph is in an atlas (`x`, `y`, `width`, `height`), and where
/// its box goes from the glyph's origin: `left` pixels to the right, `top`
/// pixels above the baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphBox {
    pub id: u16,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
}

/// Glyphs drawn as distances into one image: four bytes a pixel, all
/// four the same distance (spec 14.31).
#[derive(Clone, Debug, PartialEq)]
pub struct Atlas {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub spread: f64,
    pub boxes: Vec<GlyphBox>,
}

/// How far from a glyph's edge its distances reach, for `size`.
pub fn spread_for(size: f64) -> f64 {
    (size / 8.0).max(2.0)
}

/// The atlas of every glyph that shaping `text` uses, at `size` pixels to
/// the em; or why there is none (too big). An empty one for bytes that
/// aren't a font, as `shape` gives nothing.
pub fn atlas(bytes: &[u8], text: &str, size: f64) -> Result<Atlas, String> {
    let spread = spread_for(size);
    let Ok(f) = face(bytes) else {
        return sdf::pack(Vec::new(), spread);
    };
    let scale = size / f64::from(f.units_per_em().max(1));
    let mut ids: Vec<u16> = Vec::new();
    for g in shape(bytes, text, size).0 {
        if !ids.contains(&g.id) {
            ids.push(g.id);
        }
    }
    let drawn: Vec<sdf::Drawn> = ids
        .iter()
        .filter_map(|id| sdf::draw(&f, GlyphId(*id), scale, spread))
        .collect();
    sdf::pack(drawn, spread)
}

fn kept() -> MutexGuard<'static, (HashMap<i64, Atlas>, i64)> {
    static KEPT: OnceLock<Mutex<(HashMap<i64, Atlas>, i64)>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Makes the atlas and keeps it: its number.
pub fn atlas_kept(bytes: &[u8], text: &str, size: f64) -> Result<i64, String> {
    let made = atlas(bytes, text, size)?;
    let mut k = kept();
    k.1 += 1;
    let n = k.1;
    k.0.insert(n, made);
    Ok(n)
}

/// Atlas `n`'s width and height, then seven numbers a box (glyph, `x`,
/// `y`, `width`, `height`, `left`, `top`); nothing once it is gone.
pub fn atlas_boxes(n: i64) -> Vec<i64> {
    let k = kept();
    let Some(a) = k.0.get(&n) else {
        return Vec::new();
    };
    let mut out = vec![i64::from(a.width), i64::from(a.height)];
    for b in &a.boxes {
        out.extend([
            i64::from(b.id),
            i64::from(b.x),
            i64::from(b.y),
            i64::from(b.width),
            i64::from(b.height),
            i64::from(b.left),
            i64::from(b.top),
        ]);
    }
    out
}

/// Atlas `n`'s pixels, taking them: the atlas is let go.
pub fn take_atlas_pixels(n: i64) -> Vec<u8> {
    kept().0.remove(&n).map(|a| a.rgba).unwrap_or_default()
}
