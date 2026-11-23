// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The sample font (`sample.rs`) read, shaped, drawn as distances and
//! as coverage, its colour glyph in colour, glyphs added to an atlas, and
//! found as an installed font; and what isn't a font refused.

use crate::sample::sample_font;
use crate::{atlas, atlas_add, metrics, shape, spread_for};

#[test]
fn the_sample_font_file_is_current() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/sample.ttf");
    let made = sample_font();
    if std::fs::read(path).ok().as_ref() != Some(&made) {
        std::fs::write(path, &made).unwrap();
        panic!("src/sample.ttf was out of date and has been written again");
    }
}

#[test]
fn a_font_gives_its_metrics_in_ems() {
    assert_eq!(metrics(&sample_font()).unwrap(), [0.8, -0.2, 0.1]);
}

#[test]
fn text_is_shaped_into_placed_glyphs() {
    let (glyphs, width) = shape(&sample_font(), "AB A", 100.0);
    let placed: Vec<(u16, f64, f64)> = glyphs.iter().map(|g| (g.id, g.x, g.y)).collect();
    assert_eq!(
        placed,
        vec![
            (1, 0.0, 0.0),
            (3, 70.0, 0.0),
            (2, 140.0, 0.0),
            (1, 170.0, 0.0)
        ]
    );
    assert_eq!(width, 240.0);
    // Each glyph's cluster (a character of the text) and advance.
    let more: Vec<(usize, f64)> = glyphs.iter().map(|g| (g.cluster, g.advance)).collect();
    assert_eq!(more, vec![(0, 70.0), (1, 70.0), (2, 30.0), (3, 70.0)]);
    // Clusters count characters, not bytes.
    let after = shape(&sample_font(), "éA", 100.0).0;
    assert_eq!(
        after.iter().map(|g| g.cluster).collect::<Vec<_>>(),
        vec![0, 1]
    );
    // A character the font lacks is glyph 0, as HarfBuzz gives it.
    assert_eq!(shape(&sample_font(), "z", 10.0).0[0].id, 0);
}

#[test]
fn an_atlas_holds_distances_with_holes_outside() {
    let a = atlas(&sample_font(), "A BA", 64.0, false).unwrap();
    assert_eq!(a.spread, spread_for(64.0));
    // The space has nothing to draw.
    let ids: Vec<u16> = a.boxes.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![1, 3]);
    let b = a.boxes[0];
    // 500 units at 0.064 pixels each, and 8 pixels of spread each side.
    assert_eq!((b.width, b.height, b.left, b.top), (49, 61, -2, 53));
    let at = |bx: &crate::GlyphBox, col: u32, row: u32| {
        a.rgba[(((bx.y + row) * a.width + bx.x + col) * 4) as usize]
    };
    assert_eq!(at(&b, 24, 30), 255, "deep inside the square");
    assert_eq!(at(&b, 0, 0), 0, "far outside it");
    let edge = at(&b, 8, 30);
    assert!((112..=144).contains(&edge), "on the edge: {edge}");
    let hole = a.boxes[1];
    assert!(at(&hole, 24, 30) < 128, "the hole of B is outside");
    assert!(at(&hole, 11, 30) > 128, "its rim is inside");
}

#[test]
fn what_is_not_a_font_is_refused() {
    assert!(metrics(b"hello").unwrap_err().contains("not an OpenType"));
    let empty = atlas(b"hello", "A", 10.0, false).unwrap();
    assert_eq!((empty.width, empty.height, empty.boxes.len()), (1, 1, 0));
    assert_eq!(shape(b"hello", "A", 10.0).0, vec![]);
}

#[test]
fn an_atlas_too_big_is_refused() {
    let wide = crate::sdf::Drawn::blank(1, 5000, 10);
    let why = crate::sdf::pack(vec![wide], 2.0).unwrap_err();
    assert!(why.contains("more than 4096"), "{why}");
}

#[test]
fn an_exact_atlas_holds_coverage_and_colours() {
    // The square spans 1.5 to 9 pixels across at 15 pixels to the em.
    let a = atlas(&sample_font(), "A", 15.0, true).unwrap();
    let b = a.boxes[0];
    assert_eq!(
        (b.width, b.height, b.left, b.top, b.color),
        (8, 11, 1, 11, false)
    );
    let at = |bx: &crate::GlyphBox, col: u32, row: u32| {
        let k = (((bx.y + row) * a.width + bx.x + col) * 4) as usize;
        [a.rgba[k], a.rgba[k + 1], a.rgba[k + 2], a.rgba[k + 3]]
    };
    assert_eq!(at(&b, 4, 5), [255; 4], "inside");
    assert_eq!(at(&b, 0, 5), [127; 4], "half the pixel is covered");
    assert_eq!(at(&b, 0, 0)[3], 63, "a quarter of the corner pixel");
    // B is a colour glyph: a red square, its hole not cut out.
    let c = atlas(&sample_font(), "B", 15.0, true).unwrap();
    let red = c.boxes[0];
    assert!(red.color);
    let k = (((red.y + 5) * c.width + red.x + 4) * 4) as usize;
    assert_eq!(c.rgba[k..k + 4], [255, 0, 0, 255]);
    // In a distance atlas it has its outline, as any glyph.
    assert!(!atlas(&sample_font(), "B", 15.0, false).unwrap().boxes[0].color);
}

#[test]
fn glyphs_are_added_below_an_atlas() {
    let font = sample_font();
    let a = atlas(&font, "A", 64.0, false).unwrap();
    let b = atlas_add(a.clone(), &font, "BA", 64.0, false).unwrap();
    assert_eq!(b.boxes.len(), 2);
    assert_eq!(b.boxes[0], a.boxes[0], "A stays where it was");
    assert!(b.boxes[1].y > a.height, "B is below it");
    assert_eq!(b.width, a.width);
    let row = (a.width * 4) as usize;
    assert_eq!(b.rgba[..a.rgba.len()], a.rgba[..], "its pixels too");
    assert_eq!(&b.rgba[..row], &a.rgba[..row]);
    // Nothing to add: the same atlas.
    let same = atlas_add(b.clone(), &font, "AB", 64.0, false).unwrap();
    assert_eq!(
        (same.width, same.height, same.boxes),
        (b.width, b.height, b.boxes)
    );
}

#[test]
fn installed_fonts_are_found_by_name_and_glyphs() {
    let here = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let fonts = crate::system::scan(&[here]);
    assert_eq!(fonts.len(), 1);
    assert_eq!(
        (fonts[0].family.as_str(), fonts[0].style.as_str()),
        ("Kavach Sample", "Regular")
    );
    assert!(fonts[0].path.ends_with("sample.ttf"));
    let fallback = |text| crate::system::fallback_in(&fonts, &["Kavach Sample"], text);
    assert_eq!(fallback("A B"), Some(0), "spaces need no glyph");
    assert_eq!(fallback("Az"), None, "no glyph for z");
    // Only an installed font's file is read.
    assert!(crate::read_installed("Cargo.toml").is_empty());
}
