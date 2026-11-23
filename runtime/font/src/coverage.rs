// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Glyphs drawn for one size (spec 14.31): each pixel holds how much of
//! it the glyph covers, from 8 × 8 points in it, so small text stays
//! sharp where distances blur. A colour glyph (an emoji of a `COLR`
//! font) is drawn in its colours, layer over layer.

use crate::sdf::{Drawn, inside, outline};
use rustybuzz::ttf_parser::{self, GlyphId, RgbaColor, colr};

/// Points a pixel is tried at, along each side.
const STEPS: u32 = 8;

/// The box of a glyph whose outline spans `rect` (font units) at
/// `scale`: its left, top, width and height in pixels.
fn bounds(rect: (f64, f64, f64, f64), scale: f64) -> (f64, f64, u32, u32) {
    let left = (rect.0 * scale).floor();
    let top = (rect.3 * scale).ceil();
    let width = ((rect.2 * scale).ceil() - left).max(1.0) as u32;
    let height = (top - (rect.1 * scale).floor()).max(1.0) as u32;
    (left, top, width, height)
}

/// The points of pixel `(col, row)` of a box at `left`, `top`, `y`
/// upwards.
fn points(left: f64, top: f64, col: u32, row: u32) -> impl Iterator<Item = (f64, f64)> {
    let step = 1.0 / f64::from(STEPS);
    (0..STEPS * STEPS).map(move |k| {
        let (i, j) = (k % STEPS, k / STEPS);
        (
            left + f64::from(col) + (f64::from(i) + 0.5) * step,
            top - f64::from(row) - (f64::from(j) + 0.5) * step,
        )
    })
}

/// Glyph `id` drawn as coverage at `scale` pixels a font unit, in its
/// colours when it is a colour glyph; `None` when it has nothing to draw.
pub(crate) fn draw(face: &ttf_parser::Face, id: GlyphId, scale: f64) -> Option<Drawn> {
    if face.is_color_glyph(id)
        && let Some(d) = draw_colors(face, id, scale)
    {
        return Some(d);
    }
    let (lines, rect) = outline(face, id, scale)?;
    let r = [rect.x_min, rect.y_min, rect.x_max, rect.y_max].map(f64::from);
    let (left, top, width, height) = bounds((r[0], r[1], r[2], r[3]), scale);
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for row in 0..height {
        for col in 0..width {
            let hits = points(left, top, col, row)
                .filter(|&(x, y)| inside(&lines, x, y))
                .count();
            pixels.push((hits * 255 / (STEPS * STEPS) as usize) as u8);
        }
    }
    Some(Drawn {
        id: id.0,
        width,
        height,
        left: left as i32,
        top: top as i32,
        pixels,
        colors: None,
    })
}

/// One fill of a colour glyph: where (lines in pixels, `y` upwards, all
/// of which must hold the point), and its colour.
struct Fill {
    shapes: Vec<Vec<[f64; 4]>>,
    color: [f64; 4],
}

/// Gathers a colour glyph's fills, in pixels.
struct Painter<'f, 'd> {
    face: &'f ttf_parser::Face<'d>,
    scale: f64,
    transforms: Vec<ttf_parser::Transform>,
    /// The outline last given, until it becomes a clip.
    outline: Option<Vec<[f64; 4]>>,
    clips: Vec<Vec<[f64; 4]>>,
    fills: Vec<Fill>,
}

impl Painter<'_, '_> {
    /// `(x, y)` in font units, through the transforms, in pixels.
    fn place(&self, x: f64, y: f64) -> (f64, f64) {
        let (mut x, mut y) = (x, y);
        for t in self.transforms.iter().rev() {
            let [a, b, c, d] = [t.a, t.b, t.c, t.d].map(f64::from);
            (x, y) = (
                a * x + c * y + f64::from(t.e),
                b * x + d * y + f64::from(t.f),
            );
        }
        (x * self.scale, y * self.scale)
    }

    fn color(c: RgbaColor) -> [f64; 4] {
        [c.red, c.green, c.blue, c.alpha].map(|v| f64::from(v) / 255.0)
    }
}

impl<'a> colr::Painter<'a> for Painter<'_, '_> {
    fn outline_glyph(&mut self, glyph_id: GlyphId) {
        // In font units, then placed through the transforms.
        self.outline = outline(self.face, glyph_id, 1.0).map(|(lines, _)| {
            (lines.iter())
                .map(|&[ax, ay, bx, by]| {
                    let (a, b) = (self.place(ax, ay), self.place(bx, by));
                    [a.0, a.1, b.0, b.1]
                })
                .collect()
        });
    }

    fn paint(&mut self, paint: colr::Paint<'a>) {
        let color = match paint {
            colr::Paint::Solid(c) => Self::color(c),
            // Gradients are drawn in the mean of their stops' colours.
            colr::Paint::LinearGradient(g) => mean(g.stops(0, &[])),
            colr::Paint::RadialGradient(g) => mean(g.stops(0, &[])),
            colr::Paint::SweepGradient(g) => mean(g.stops(0, &[])),
        };
        let mut shapes = self.clips.clone();
        shapes.extend(self.outline.clone());
        if !shapes.is_empty() {
            self.fills.push(Fill { shapes, color });
        }
    }

    fn push_clip(&mut self) {
        let shape = self.outline.take().unwrap_or_default();
        self.clips.push(shape);
    }

    fn push_clip_box(&mut self, b: colr::ClipBox) {
        let [x0, y0, x1, y1] = [b.x_min, b.y_min, b.x_max, b.y_max].map(f64::from);
        let c = [
            self.place(x0, y0),
            self.place(x1, y0),
            self.place(x1, y1),
            self.place(x0, y1),
        ];
        let lines = (0..4)
            .map(|k| [c[k].0, c[k].1, c[(k + 1) % 4].0, c[(k + 1) % 4].1])
            .collect();
        self.clips.push(lines);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn push_layer(&mut self, _: colr::CompositeMode) {}

    fn pop_layer(&mut self) {}

    fn push_transform(&mut self, t: ttf_parser::Transform) {
        self.transforms.push(t);
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
    }
}

/// The mean of a gradient's colours.
fn mean(stops: impl Iterator<Item = colr::ColorStop>) -> [f64; 4] {
    let (mut sum, mut n) = ([0.0; 4], 0.0);
    for s in stops {
        let c = Painter::color(s.color);
        (0..4).for_each(|k| sum[k] += c[k]);
        n += 1.0;
    }
    if n == 0.0 { sum } else { sum.map(|v| v / n) }
}

/// A colour glyph drawn in its palette's first colours: each pixel the
/// fills over each other, as straight (not premultiplied) colours.
fn draw_colors(face: &ttf_parser::Face, id: GlyphId, scale: f64) -> Option<Drawn> {
    let mut p = Painter {
        face,
        scale,
        transforms: Vec::new(),
        outline: None,
        clips: Vec::new(),
        fills: Vec::new(),
    };
    let black = RgbaColor::new(0, 0, 0, 255);
    face.paint_color_glyph(id, 0, black, &mut p)?;
    // The box: what the fills' shapes span.
    let all = (p.fills.iter())
        .flat_map(|f| f.shapes.iter().flatten())
        .flat_map(|l| [(l[0], l[1]), (l[2], l[3])]);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in all {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    if x0 > x1 {
        return None;
    }
    let (left, top, width, height) = bounds((x0, y0, x1, y1), 1.0);
    let mut colors = Vec::with_capacity((width * height * 4) as usize);
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for row in 0..height {
        for col in 0..width {
            // Premultiplied while adding up, straight when stored.
            let mut sum = [0.0f64; 4];
            for (x, y) in points(left, top, col, row) {
                let mut c = [0.0f64; 4];
                for f in &p.fills {
                    if f.shapes.iter().all(|s| inside(s, x, y)) {
                        let a = f.color[3];
                        for (part, colour) in c.iter_mut().zip(f.color).take(3) {
                            *part = colour * a + *part * (1.0 - a);
                        }
                        c[3] = a + c[3] * (1.0 - a);
                    }
                }
                (0..4).for_each(|k| sum[k] += c[k]);
            }
            let n = f64::from(STEPS * STEPS);
            let alpha = sum[3] / n;
            for part in &sum[..3] {
                let straight = if alpha > 0.0 { part / n / alpha } else { 0.0 };
                colors.push((straight * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            let a = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            colors.push(a);
            pixels.push(a);
        }
    }
    Some(Drawn {
        id: id.0,
        width,
        height,
        left: left as i32,
        top: top as i32,
        pixels,
        colors: Some(colors),
    })
}
