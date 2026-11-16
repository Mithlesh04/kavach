//! Glyphs drawn as distances (spec 14.31): each glyph's outline cut into
//! straight lines, each pixel given its distance to the nearest line,
//! inside or out by the outline's winding, and the glyphs packed into
//! one image in shelves.

use crate::{Atlas, GlyphBox, MAX_ATLAS};
use rustybuzz::ttf_parser::{self, GlyphId, OutlineBuilder};

/// Lines a curve is cut into: enough that the error is far below a pixel
/// at the sizes atlases are made at.
const PIECES: usize = 8;

/// One glyph drawn: its distances, a byte a pixel, and its box.
pub(crate) struct Drawn {
    id: u16,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    pixels: Vec<u8>,
}

#[cfg(test)]
impl Drawn {
    /// A glyph of `width` × `height` pixels, all outside.
    pub(crate) fn blank(id: u16, width: u32, height: u32) -> Drawn {
        let pixels = vec![0; (width * height) as usize];
        let (left, top) = (0, 0);
        Drawn {
            id,
            width,
            height,
            left,
            top,
            pixels,
        }
    }
}

/// An outline as straight lines, scaled to pixels, `y` upwards.
struct Lines {
    scale: f64,
    start: (f64, f64),
    at: (f64, f64),
    lines: Vec<[f64; 4]>,
}

impl Lines {
    fn to(&mut self, x: f64, y: f64) {
        self.lines.push([self.at.0, self.at.1, x, y]);
        self.at = (x, y);
    }

    fn point(&self, x: f32, y: f32) -> (f64, f64) {
        (f64::from(x) * self.scale, f64::from(y) * self.scale)
    }
}

impl OutlineBuilder for Lines {
    fn move_to(&mut self, x: f32, y: f32) {
        self.at = self.point(x, y);
        self.start = self.at;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.to(p.0, p.1);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b, c) = (self.at, self.point(x1, y1), self.point(x, y));
        for i in 1..=PIECES {
            let t = i as f64 / PIECES as f64;
            let u = 1.0 - t;
            let q = |p: f64, q: f64, r: f64| u * u * p + 2.0 * u * t * q + t * t * r;
            self.to(q(a.0, b.0, c.0), q(a.1, b.1, c.1));
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let a = self.at;
        let (b, c, d) = (self.point(x1, y1), self.point(x2, y2), self.point(x, y));
        for i in 1..=PIECES {
            let t = i as f64 / PIECES as f64;
            let u = 1.0 - t;
            let k = |p: f64, q: f64, r: f64, s: f64| {
                u * u * u * p + 3.0 * u * u * t * q + 3.0 * u * t * t * r + t * t * t * s
            };
            self.to(k(a.0, b.0, c.0, d.0), k(a.1, b.1, c.1, d.1));
        }
    }

    fn close(&mut self) {
        if self.at != self.start {
            self.to(self.start.0, self.start.1);
        }
    }
}

/// The distance from `(x, y)` to the nearest line, positive inside the
/// outline (its winding not zero) and negative outside.
fn signed_distance(lines: &[[f64; 4]], x: f64, y: f64) -> f64 {
    let mut nearest = f64::INFINITY;
    let mut winding = 0;
    for &[ax, ay, bx, by] in lines {
        let (dx, dy) = (bx - ax, by - ay);
        let length = dx * dx + dy * dy;
        let t = if length > 0.0 {
            (((x - ax) * dx + (y - ay) * dy) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (px, py) = (ax + t * dx - x, ay + t * dy - y);
        nearest = nearest.min(px * px + py * py);
        let cross = dx * (y - ay) - dy * (x - ax);
        if ay <= y && by > y && cross > 0.0 {
            winding += 1;
        } else if by <= y && ay > y && cross < 0.0 {
            winding -= 1;
        }
    }
    let d = nearest.sqrt();
    if winding != 0 { d } else { -d }
}

/// Glyph `id` drawn as distances at `scale` pixels a font unit, reaching
/// `spread` pixels from its edge; `None` when it has no outline.
pub(crate) fn draw(face: &ttf_parser::Face, id: GlyphId, scale: f64, spread: f64) -> Option<Drawn> {
    let mut lines = Lines {
        scale,
        start: (0.0, 0.0),
        at: (0.0, 0.0),
        lines: Vec::new(),
    };
    let rect = face.outline_glyph(id, &mut lines)?;
    if lines.lines.is_empty() {
        return None;
    }
    let pad = spread.ceil();
    let left = (f64::from(rect.x_min) * scale - pad).floor();
    let top = (f64::from(rect.y_max) * scale + pad).ceil();
    let width = ((f64::from(rect.x_max) * scale + pad).ceil() - left).max(1.0) as u32;
    let height = (top - (f64::from(rect.y_min) * scale - pad).floor()).max(1.0) as u32;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for row in 0..height {
        let y = top - f64::from(row) - 0.5;
        for col in 0..width {
            let x = left + f64::from(col) + 0.5;
            let d = signed_distance(&lines.lines, x, y);
            pixels.push((128.0 + d * 128.0 / spread).clamp(0.0, 255.0) as u8);
        }
    }
    Some(Drawn {
        id: id.0,
        width,
        height,
        left: left as i32,
        top: top as i32,
        pixels,
    })
}

/// The glyphs packed into one image, in shelves from the tallest, a pixel
/// apart; or why they don't fit in `MAX_ATLAS` pixels a side.
pub(crate) fn pack(mut drawn: Vec<Drawn>, spread: f64) -> Result<Atlas, String> {
    drawn.sort_by(|a, b| b.height.cmp(&a.height).then(a.id.cmp(&b.id)));
    let area: u64 = drawn
        .iter()
        .map(|d| u64::from(d.width + 1) * u64::from(d.height + 1))
        .sum();
    let widest = drawn.iter().map(|d| d.width + 1).max().unwrap_or(1);
    let side = ((area as f64).sqrt() * 1.1).ceil() as u32;
    let width = side
        .max(widest)
        .next_power_of_two()
        .clamp(1, MAX_ATLAS.max(widest));
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    let mut places = Vec::with_capacity(drawn.len());
    for d in &drawn {
        if x > 0 && x + d.width > width {
            (x, y, shelf) = (0, y + shelf + 1, 0);
        }
        places.push((x, y));
        x += d.width + 1;
        shelf = shelf.max(d.height);
    }
    let height = (y + shelf).max(1);
    if width > MAX_ATLAS || height > MAX_ATLAS {
        return Err(format!(
            "the atlas would be {width} × {height} pixels, more than {MAX_ATLAS} a side: make it smaller or from less text"
        ));
    }
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let mut boxes = Vec::with_capacity(drawn.len());
    for (d, &(bx, by)) in drawn.iter().zip(&places) {
        for row in 0..d.height {
            for col in 0..d.width {
                let v = d.pixels[(row * d.width + col) as usize];
                let at = (((by + row) * width + bx + col) * 4) as usize;
                rgba[at..at + 4].fill(v);
            }
        }
        boxes.push(GlyphBox {
            id: d.id,
            x: bx,
            y: by,
            width: d.width,
            height: d.height,
            left: d.left,
            top: d.top,
        });
    }
    boxes.sort_by_key(|b| b.id);
    Ok(Atlas {
        width,
        height,
        rgba,
        spread,
        boxes,
    })
}
