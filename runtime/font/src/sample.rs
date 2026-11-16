//! A tiny TrueType font made by our own code, for tests here and in
//! `tools/cli`: 1,000 units to the em, and four glyphs: 0 (`.notdef`,
//! empty), 1 a square for `A`, 2 an empty space for ` `, and 3 a square
//! with a square hole for `B`. `src/sample.ttf` is this font, written by
//! `tests::the_sample_font_file_is_current`.

/// A glyph: its advance, and its contours, each a list of on-curve
/// points.
struct Glyph {
    advance: u16,
    contours: Vec<Vec<(i16, i16)>>,
}

fn glyphs() -> Vec<Glyph> {
    let square = vec![(100, 0), (100, 700), (600, 700), (600, 0)];
    // The hole runs the other way round, so its winding is zero.
    let hole = vec![(250, 200), (450, 200), (450, 500), (250, 500)];
    vec![
        Glyph {
            advance: 500,
            contours: vec![],
        },
        Glyph {
            advance: 700,
            contours: vec![square.clone()],
        },
        Glyph {
            advance: 300,
            contours: vec![],
        },
        Glyph {
            advance: 700,
            contours: vec![square, hole],
        },
    ]
}

fn u16s(out: &mut Vec<u8>, values: &[u16]) {
    for v in values {
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn glyph_data(g: &Glyph) -> Vec<u8> {
    if g.contours.is_empty() {
        return Vec::new();
    }
    let points: Vec<(i16, i16)> = g.contours.concat();
    let (xs, ys): (Vec<i16>, Vec<i16>) = points.iter().copied().unzip();
    let mut out = Vec::new();
    let bounds = [
        g.contours.len() as i16,
        *xs.iter().min().unwrap_or(&0),
        *ys.iter().min().unwrap_or(&0),
        *xs.iter().max().unwrap_or(&0),
        *ys.iter().max().unwrap_or(&0),
    ];
    u16s(&mut out, &bounds.map(|v| v as u16));
    let mut end = 0u16;
    for c in &g.contours {
        end += c.len() as u16;
        u16s(&mut out, &[end - 1]);
    }
    u16s(&mut out, &[0]); // no instructions
    // Each point on the curve, its two coordinates as 16-bit changes.
    out.extend(std::iter::repeat_n(1u8, points.len()));
    let deltas = |values: &[i16]| {
        let mut last = 0i16;
        values
            .iter()
            .map(|v| {
                let d = v - last;
                last = *v;
                d as u16
            })
            .collect::<Vec<u16>>()
    };
    u16s(&mut out, &deltas(&xs));
    u16s(&mut out, &deltas(&ys));
    if out.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// A format 4 `cmap`: `A` to glyph 1, ` ` to 2, `B` to 3.
fn cmap() -> Vec<u8> {
    let pairs: [(u16, u16); 3] = [(0x20, 2), (0x41, 1), (0x42, 3)];
    let segs = pairs.len() as u16 + 1;
    let mut sub = Vec::new();
    let length = 16 + 8 * segs;
    u16s(&mut sub, &[4, length, 0, segs * 2, 8, 2, segs * 2 - 8]);
    let ends: Vec<u16> = pairs.iter().map(|p| p.0).chain([0xFFFF]).collect();
    u16s(&mut sub, &ends);
    u16s(&mut sub, &[0]);
    u16s(&mut sub, &ends); // each segment is one character
    let deltas: Vec<u16> = (pairs.iter().map(|(c, g)| g.wrapping_sub(*c)))
        .chain([1])
        .collect();
    u16s(&mut sub, &deltas);
    u16s(&mut sub, &vec![0; segs as usize]);
    let mut out = Vec::new();
    u16s(&mut out, &[0, 1, 3, 1, 0, 12]);
    out.extend(sub);
    out
}

/// The font file's bytes.
pub fn sample_font() -> Vec<u8> {
    let glyphs = glyphs();
    let (mut glyf, mut loca) = (Vec::new(), Vec::new());
    for g in &glyphs {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        glyf.extend(glyph_data(g));
    }
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    let n = glyphs.len() as u16;
    let mut head = Vec::new();
    u16s(&mut head, &[1, 0, 1, 0, 0, 0, 0x5F0F, 0x3CF5, 0, 1000]);
    head.extend([0; 16]); // created and modified
    u16s(&mut head, &[100, 0, 600, 700, 0, 8, 2, 1, 0]);
    let mut hhea = Vec::new();
    u16s(
        &mut hhea,
        &[1, 0, 800, (-200i16) as u16, 100, 700, 100, 100, 600],
    );
    u16s(&mut hhea, &[1, 0, 0, 0, 0, 0, 0, 0, n]);
    let mut hmtx = Vec::new();
    for g in &glyphs {
        u16s(&mut hmtx, &[g.advance, 100]);
    }
    let mut maxp = Vec::new();
    u16s(&mut maxp, &[0, 0x5000, n]);
    let tables: [(&[u8; 4], Vec<u8>); 7] = [
        (b"cmap", cmap()),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca),
        (b"maxp", maxp),
    ];
    let mut out = Vec::new();
    u16s(&mut out, &[1, 0, tables.len() as u16, 64, 2, 48]);
    let mut offset = 12 + 16 * tables.len() as u32;
    let mut body = Vec::new();
    for (tag, data) in &tables {
        out.extend_from_slice(*tag);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&offset.to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        body.extend_from_slice(data);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        offset = 12 + 16 * tables.len() as u32 + body.len() as u32;
    }
    out.extend(body);
    out
}
