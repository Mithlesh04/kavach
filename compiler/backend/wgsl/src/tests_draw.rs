//! Depth and textures (spec 15.3), written and drawn off-screen. The
//! drawing tests are skipped on a machine with no GPU.

use crate::tests::{drawing, one, pipeline_of};
use kavach_runtime_gpu::render::{self, Draw, Items::Words};

/// A corner at a depth, its color from `heat`, through a pipeline with
/// depth.
const DEEP: &str = "use std.gpu

type Corner {
    x: Float32
    y: Float32
    z: Float32
    heat: Float32
}

type Lit {
    position: (Float32, Float32, Float32, Float32)
    heat: Float32
}

gpu vertex fn place(c: Corner) -> Lit {
    Lit(position: (c.x, c.y, c.z, 1.0), heat: c.heat)
}

gpu fragment fn glow(l: Lit) -> (Float32, Float32, Float32, Float32) {
    (l.heat, 0.0, 1.0 - l.heat, 1.0)
}

fn main() {
    const _ = gpu.pipeline(place, glow, depth: true)
}
";

/// A square over the whole picture, painted from a texture.
const PAINTED: &str = "use std.gpu

type Corner {
    x: Float32
    y: Float32
}

type Spot {
    position: (Float32, Float32, Float32, Float32)
    u: Float32
    v: Float32
}

gpu vertex fn place(c: Corner) -> Spot {
    Spot(position: (c.x, c.y, 0.0, 1.0), u: (c.x + 1.0) / 2.0, v: (1.0 - c.y) / 2.0)
}

gpu fragment fn paint(s: Spot, skin: Texture) -> (Float32, Float32, Float32, Float32) {
    skin.sample(s.u, s.v)
}

fn main() {
    const _ = gpu.pipeline(place, paint)
}
";

fn words(fs: &[f32]) -> Vec<u32> {
    fs.iter().map(|f| f.to_bits()).collect()
}

#[test]
fn depth_and_textures_are_in_the_first_line() {
    let deep = pipeline_of(DEEP);
    assert!(
        deep.starts_with("// draw f f f f |  | 0 | 1 | 0\n"),
        "{deep}"
    );
    let painted = pipeline_of(PAINTED);
    assert!(
        painted.starts_with("// draw f f |  | 0 | 0 | 1\n"),
        "{painted}"
    );
    for part in [
        "@group(1) @binding(0) var ksampler: sampler;",
        "@group(1) @binding(1) var ktex0: texture_2d<f32>;",
        "textureSampleLevel(t, ksampler",
        ", ktex0);",
    ] {
        assert!(painted.contains(part), "{part} missing in {painted}");
    }
}

/// A triangle over the middle at depth `z`, red when `heat` is 1.
fn triangle(z: f32, heat: f32) -> Vec<u32> {
    words(&[-0.8, -0.8, z, heat, 0.8, -0.8, z, heat, 0.0, 0.8, z, heat])
}

/// The nearer of two triangles is drawn, whichever is drawn first; with
/// depth, the farther one doesn't cover it.
#[test]
fn a_pipeline_with_depth_draws_the_nearer_pixel() {
    let _alone = drawing();
    if !kavach_runtime_gpu::available() {
        return;
    }
    let n = render::pipeline(&pipeline_of(DEEP)).expect("a layout");
    assert!(render::has_depth(n));
    let (near, far) = (triangle(0.2, 1.0), triangle(0.8, 0.0));
    for order in [[&near, &far], [&far, &near]] {
        let draws = order.iter().map(|t| one(n, t, &[], &[])).collect();
        let pixels = render::picture(draws, (16, 16)).expect("drawn");
        assert_eq!(pixels[8 * 16 + 8], [255, 0, 0, 255], "the near one, red");
        assert_eq!(pixels[0], [0, 0, 0, 255], "nothing drawn there");
    }
}

/// Two pixels, red then blue, sampled across the picture: each side
/// takes its pixel, and between them the two blend.
#[test]
fn a_fragment_fn_samples_its_texture() {
    let _alone = drawing();
    if !kavach_runtime_gpu::available() {
        return;
    }
    use kavach_runtime_gpu::texture;
    let n = render::pipeline(&pipeline_of(PAINTED)).expect("a layout");
    let skin = texture::texture(2, 1, &[255, 0, 0, 255, 0, 0, 255, 255]).expect("a texture");
    let square = words(&[
        -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
    ]);
    let draw = Draw {
        pipeline: n,
        parts: (Words(&square), Words(&[]), &[]),
        textures: &[skin],
    };
    let pixels = render::picture(vec![draw], (4, 4)).expect("drawn");
    assert_eq!(pixels[4], [255, 0, 0, 255], "left, the red pixel");
    assert_eq!(pixels[7], [0, 0, 255, 255], "right, the blue pixel");
    let [r, g, b, a] = pixels[5];
    assert!(
        r > 150 && r < 230 && g == 0 && b > 30 && b < 100 && a == 255,
        "{r} {g} {b}"
    );
    // A draw giving another number of textures is refused.
    let none = Draw {
        pipeline: n,
        parts: (Words(&square), Words(&[]), &[]),
        textures: &[],
    };
    assert!(render::picture(vec![none], (4, 4)).is_err());
    texture::drop_texture(skin);
}
