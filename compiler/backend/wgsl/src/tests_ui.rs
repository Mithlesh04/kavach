// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Drawing for user interfaces (spec 14.24.1, 15.3), written and drawn
//! off-screen: the shared value in the fragment fn, `fwidth` and
//! `smoothstep`, the mask, the clip and premultiplied blending. The
//! drawing tests are skipped on a machine with no GPU.

use crate::tests::{drawing, one, pipeline_of};
use kavach_runtime_gpu::render::{self, Mask, State};

/// A triangle over the whole picture, its color the draw's shared value,
/// read by the fragment fn alone, softened by `fwidth` and `smoothstep`.
const TINTED: &str = "use std.gpu
use std.math

type Corner {
    x: Float32
    y: Float32
}

type Spot {
    position: (Float32, Float32, Float32, Float32)
    u: Float32
}

type Tint {
    r: Float32
    g: Float32
    b: Float32
    a: Float32
}

gpu vertex fn place(c: Corner) -> Spot {
    Spot(position: (c.x, c.y, 0.0, 1.0), u: c.x)
}

gpu fragment fn tint(s: Spot, t: Tint) -> (Float32, Float32, Float32, Float32) {
    const soft = math.fwidth(s.u)
    const edge = math.smoothstep(-2.0 - soft, -2.0 + soft, s.u)
    (t.r * edge, t.g, math.mix(t.b, t.b, 0.5), t.a)
}

fn main() {
    const _ = gpu.pipeline(place, tint)
}
";

fn words(fs: &[f32]) -> Vec<u32> {
    fs.iter().map(|f| f.to_bits()).collect()
}

/// A triangle covering a picture.
fn cover() -> Vec<u32> {
    words(&[-1.0, -1.0, 3.0, -1.0, -1.0, 3.0])
}

#[test]
fn the_fragment_fn_reads_the_shared_value() {
    let text = pipeline_of(TINTED);
    assert!(text.starts_with("// draw f f |  | 4 | 0 | 0\n"), "{text}");
    for part in [
        "var<storage, read> shared_words",
        "fn fs_color(i: VOut) -> vec4<f32>",
        "fwidth(",
        "@fragment\nfn fs_mask(i: VOut)",
        "discard;",
    ] {
        assert!(text.contains(part), "{part} missing in {text}");
    }
    // The vertex fn doesn't take it, so `vs_main` doesn't load it.
    assert!(text.contains("let r = "), "{text}");
}

/// The shared value colors every pixel; the mask keeps a second draw
/// to the marked half; the clip to one column; premultiplied colors are
/// taken as they are.
#[test]
fn a_frame_draws_with_its_mask_clip_and_blending() {
    let _alone = drawing();
    if !kavach_runtime_gpu::available() {
        return;
    }
    let n = render::pipeline(&pipeline_of(TINTED)).expect("a layout");
    let all = cover();
    let left = words(&[
        -1.0, -1.0, 0.0, -1.0, 0.0, 1.0, -1.0, -1.0, 0.0, 1.0, -1.0, 1.0,
    ]);
    let (red, blue) = (words(&[1.0, 0.0, 0.0, 1.0]), words(&[0.0, 0.0, 1.0, 1.0]));
    let state = |mask, clip, premultiplied| State {
        clip,
        premultiplied,
        mask,
    };
    let draws = vec![
        (one(n, &left, &[], &red), state(Mask::Write, None, false)),
        (one(n, &all, &[], &red), state(Mask::Inside, None, false)),
        (
            one(n, &all, &[], &blue),
            state(Mask::Outside, Some([3, 0, 1, 4]), false),
        ),
    ];
    let pixels = render::picture_with(draws, (4, 4)).expect("drawn");
    assert_eq!(pixels[4], [255, 0, 0, 255], "marked: red");
    assert_eq!(
        pixels[4 + 2],
        [0, 0, 0, 255],
        "not marked, outside the clip"
    );
    assert_eq!(pixels[4 + 3], [0, 0, 255, 255], "not marked, in the clip");
    // Half opaque red, premultiplied or not, over black.
    let half = words(&[0.5, 0.0, 0.0, 0.5]);
    let blended = |premultiplied| {
        let d = vec![(
            one(n, &all, &[], &half),
            state(Mask::Off, None, premultiplied),
        )];
        render::picture_with(d, (1, 1)).expect("drawn")[0][0]
    };
    assert!((63..=64).contains(&blended(false)), "0.5 × 0.5 over black");
    assert!((127..=128).contains(&blended(true)), "0.5 as it is");
}
