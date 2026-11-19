// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The sample font (`sample.rs`) read, shaped and drawn as distances; and
//! what isn't a font refused.

use crate::sample::sample_font;
use crate::{atlas, metrics, shape, spread_for};

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
    // A character the font lacks is glyph 0, as HarfBuzz gives it.
    assert_eq!(shape(&sample_font(), "z", 10.0).0[0].id, 0);
}

#[test]
fn an_atlas_holds_distances_with_holes_outside() {
    let a = atlas(&sample_font(), "A BA", 64.0).unwrap();
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
    let empty = atlas(b"hello", "A", 10.0).unwrap();
    assert_eq!((empty.width, empty.height, empty.boxes.len()), (1, 1, 0));
    assert_eq!(shape(b"hello", "A", 10.0).0, vec![]);
}

#[test]
fn an_atlas_too_big_is_refused() {
    let wide = crate::sdf::Drawn::blank(1, 5000, 10);
    let why = crate::sdf::pack(vec![wide], 2.0).unwrap_err();
    assert!(why.contains("more than 4096"), "{why}");
}
