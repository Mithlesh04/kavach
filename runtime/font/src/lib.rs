// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Fonts for `std.font` (spec 14.31, decision 0121): OpenType and
//! TrueType files read through `ttf-parser`, text shaped through
//! `rustybuzz`, and glyphs drawn into an atlas by our own code: as
//! distances (`sdf.rs`), or as coverage for one size, colour glyphs in
//! their colours (`coverage.rs`). A font is its file's bytes, read again
//! at each call, which `ttf-parser` does without copying. An atlas is
//! kept by number until `std.font` has read its boxes and pixels. The
//! fonts installed on the machine are found in `system.rs`.

use rustybuzz::ttf_parser::GlyphId;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

mod coverage;
#[cfg(test)]
mod sample;
mod sdf;
mod system;
#[cfg(test)]
mod tests;

pub use system::{Installed, fallback, installed, installed_flat, read_installed};

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

/// One shaped glyph: its number, where its origin goes in pixels from
/// the start of the line, on the baseline, `y` growing downwards; the
/// character of the text its cluster starts at, and how far it moves the
/// pen, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub id: u16,
    pub x: f64,
    pub y: f64,
    pub cluster: usize,
    pub advance: f64,
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
    // HarfBuzz gives clusters as bytes of the text; Kavach counts
    // characters (spec 14.2).
    let starts: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
    let mut pen = (0.0, 0.0);
    let mut out = Vec::with_capacity(shaped.len());
    for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        out.push(Placed {
            id: info.glyph_id as u16,
            x: pen.0 + f64::from(pos.x_offset) * scale,
            y: pen.1 - f64::from(pos.y_offset) * scale,
            cluster: starts.partition_point(|&b| b < info.cluster as usize),
            advance: f64::from(pos.x_advance) * scale,
        });
        pen.0 += f64::from(pos.x_advance) * scale;
        pen.1 -= f64::from(pos.y_advance) * scale;
    }
    (out, pen.0)
}

/// `shape`'s result as `std.font` reads it: five numbers a glyph (its
/// number, `x`, `y`, cluster and advance), then the width.
pub fn shape_flat(bytes: &[u8], text: &str, size: f64) -> Vec<f64> {
    let (glyphs, width) = shape(bytes, text, size);
    let mut out: Vec<f64> = glyphs
        .iter()
        .flat_map(|g| [f64::from(g.id), g.x, g.y, g.cluster as f64, g.advance])
        .collect();
    out.push(width);
    out
}

/// Where a glyph is in an atlas (`x`, `y`, `width`, `height`), where its
/// box goes from the glyph's origin: `left` pixels to the right, `top`
/// pixels above the baseline; and whether it is drawn in its colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphBox {
    pub id: u16,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub color: bool,
}

/// Glyphs drawn into one image: four bytes a pixel, all four the same
/// distance or coverage, or a colour glyph's colours (spec 14.31).
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

/// The glyphs shaping `text` uses, but those in `skip`, each once, drawn
/// at `size` pixels to the em: as distances, or `exact`ly, as coverage.
fn drawn(bytes: &[u8], text: &str, size: f64, exact: bool, skip: &[u16]) -> Vec<sdf::Drawn> {
    let Ok(f) = face(bytes) else {
        return Vec::new();
    };
    let scale = size / f64::from(f.units_per_em().max(1));
    let mut ids: Vec<u16> = Vec::new();
    for g in shape(bytes, text, size).0 {
        if !ids.contains(&g.id) && !skip.contains(&g.id) {
            ids.push(g.id);
        }
    }
    let spread = spread_for(size);
    let draw = |id: &u16| match exact {
        true => coverage::draw(&f, GlyphId(*id), scale),
        false => sdf::draw(&f, GlyphId(*id), scale, spread),
    };
    ids.iter().filter_map(draw).collect()
}

/// The atlas of every glyph that shaping `text` uses, at `size` pixels to
/// the em, as distances or `exact`; or why there is none (too big). An
/// empty one for bytes that aren't a font, as `shape` gives nothing.
pub fn atlas(bytes: &[u8], text: &str, size: f64, exact: bool) -> Result<Atlas, String> {
    sdf::pack(drawn(bytes, text, size, exact, &[]), spread_for(size))
}

/// `old` with the glyphs of `text` it lacks added below its pixels, its
/// own staying where they are (spec 14.31).
pub fn atlas_add(
    old: Atlas,
    bytes: &[u8],
    text: &str,
    size: f64,
    exact: bool,
) -> Result<Atlas, String> {
    let have: Vec<u16> = old.boxes.iter().map(|b| b.id).collect();
    sdf::pack_below(old, drawn(bytes, text, size, exact, &have))
}

fn kept() -> MutexGuard<'static, (HashMap<i64, Atlas>, i64)> {
    static KEPT: OnceLock<Mutex<(HashMap<i64, Atlas>, i64)>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Keeps atlas `made`: its number.
pub fn keep(made: Atlas) -> i64 {
    let mut k = kept();
    k.1 += 1;
    let n = k.1;
    k.0.insert(n, made);
    n
}

/// Makes the atlas of `text` and keeps it: its number. With `base`, the
/// number of an atlas kept by `keep_base`, the glyphs it lacks are added
/// to it instead.
pub fn atlas_kept(
    bytes: &[u8],
    text: &str,
    size: f64,
    exact: bool,
    base: i64,
) -> Result<i64, String> {
    let old = kept().0.remove(&base);
    let made = match old {
        Some(old) => atlas_add(old, bytes, text, size, exact)?,
        None => atlas(bytes, text, size, exact)?,
    };
    Ok(keep(made))
}

/// Keeps an atlas a program holds, to add glyphs to: its `pixels`, and
/// in `layout` its width and height, then eight numbers a box, as
/// `atlas_boxes` gives them. Its number, or -1 when they don't agree.
pub fn keep_base(pixels: &[u8], layout: &[i64], size: f64) -> i64 {
    let (Some(&w), Some(&h)) = (layout.first(), layout.get(1)) else {
        return -1;
    };
    let fit = |v: i64| u32::try_from(v).unwrap_or(0);
    let (width, height) = (fit(w), fit(h));
    if u64::from(width) * u64::from(height) * 4 != pixels.len() as u64 {
        return -1;
    }
    let boxes = (layout[2..].as_chunks::<8>().0.iter())
        .map(|b| GlyphBox {
            id: u16::try_from(b[0]).unwrap_or(0),
            x: fit(b[1]),
            y: fit(b[2]),
            width: fit(b[3]),
            height: fit(b[4]),
            left: i32::try_from(b[5]).unwrap_or(0),
            top: i32::try_from(b[6]).unwrap_or(0),
            color: b[7] != 0,
        })
        .collect();
    keep(Atlas {
        width,
        height,
        rgba: pixels.to_vec(),
        spread: spread_for(size),
        boxes,
    })
}

/// Atlas `n`'s width and height, then eight numbers a box (glyph, `x`,
/// `y`, `width`, `height`, `left`, `top`, and 1 for a colour glyph);
/// nothing once it is gone.
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
            i64::from(b.color),
        ]);
    }
    out
}

/// Atlas `n`'s pixels, taking them: the atlas is let go.
pub fn take_atlas_pixels(n: i64) -> Vec<u8> {
    kept().0.remove(&n).map(|a| a.rgba).unwrap_or_default()
}
