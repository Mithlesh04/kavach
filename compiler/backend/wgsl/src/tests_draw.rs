//! Depth, textures and indices (spec 15.3, 14.24), written and drawn
//! off-screen. The drawing tests are skipped on a machine with no GPU.

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
        indices: None,
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
        indices: None,
    };
    assert!(render::picture(vec![none], (4, 4)).is_err());
    texture::drop_texture(skin);
}

/// A compressed texture (spec 14.24): one BC1 block of solid green,
/// uploaded as it is, samples green everywhere. Skipped on a GPU that
/// can't read BC blocks.
#[test]
fn a_compressed_texture_samples_its_blocks() {
    let _alone = drawing();
    if !kavach_runtime_gpu::available() {
        return;
    }
    use kavach_runtime_gpu::{ktx2, texture};
    // Color 0 is pure green (5:6:5 bits), color 1 black, every index 0.
    let block = vec![0xE0, 0x07, 0, 0, 0, 0, 0, 0];
    let file = ktx2::file(131, 4, 4, &[block]);
    let green = match texture::compressed(&file) {
        Ok((n, 4, 4)) => n,
        Ok(other) => panic!("a 4 × 4 texture, not {other:?}"),
        Err(("Unavailable", _)) => return,
        Err(e) => panic!("{e:?}"),
    };
    let n = render::pipeline(&pipeline_of(PAINTED)).expect("a layout");
    let square = words(&[
        -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
    ]);
    let draw = Draw {
        pipeline: n,
        parts: (Words(&square), Words(&[]), &[]),
        textures: &[green],
        indices: None,
    };
    let pixels = render::picture(vec![draw], (4, 4)).expect("drawn");
    assert!(pixels.iter().all(|p| *p == [0, 255, 0, 255]), "{pixels:?}");
    texture::drop_texture(green);
}

/// A square from four vertices and six indices covers the picture; an
/// index past the last vertex is refused before anything is drawn.
#[test]
fn indices_name_the_vertices_of_each_triangle() {
    let _alone = drawing();
    if !kavach_runtime_gpu::available() {
        return;
    }
    let n = render::pipeline(&pipeline_of(DEEP)).expect("a layout");
    let square = words(&[
        -1.0, -1.0, 0.5, 1.0, 1.0, -1.0, 0.5, 1.0, 1.0, 1.0, 0.5, 1.0, -1.0, 1.0, 0.5, 1.0,
    ]);
    let indexed = |ix: &'static [u32]| Draw {
        pipeline: n,
        parts: (Words(&square), Words(&[]), &[]),
        textures: &[],
        indices: Some(ix),
    };
    let pixels = render::picture(vec![indexed(&[0, 1, 2, 0, 2, 3])], (8, 8)).expect("drawn");
    assert!(
        pixels.iter().all(|p| *p == [255, 0, 0, 255]),
        "every pixel red"
    );
    let err = render::picture(vec![indexed(&[0, 1, 4])], (8, 8)).expect_err("refused");
    assert!(
        err.contains("index 4 is past the last of the 4 vertices"),
        "{err}"
    );
}

/// `std.vec` in a pipeline: a `Vec4` position from a `Mat4`, and a `Vec4`
/// color, as WGSL's own types and operations (spec 14.28).
const VEC: &str = "use std.gpu
use std.vec

type Corner {
    x: Float32
    y: Float32
}

type Lit {
    position: Vec4<Float32>
    heat: Float32
}

gpu vertex fn place(c: Corner) -> Lit {
    const half = Mat4.scaling(Vec3(x: Float32(0.5), y: 0.5, z: 1.0))
    const p = Vec4(x: c.x, y: c.y, z: Float32(0.0), w: 1.0)
    Lit(position: half @ p, heat: Vec2(x: c.x, y: c.y).dot(Vec2(x: c.x, y: c.y)))
}

gpu fragment fn glow(l: Lit) -> Vec4<Float32> {
    Vec4(x: Float32(1.0), y: 0.0, z: 0.0, w: 1.0) * (l.heat - l.heat + 1.0)
}

fn main() {
    const _ = gpu.pipeline(place, glow)
}
";

#[test]
fn std_vec_is_wgsls_own_types_in_a_pipeline() {
    let text = pipeline_of(VEC);
    for part in [
        "vec4<f32>",
        "mat4x4<f32>",
        "dot(",
        "o.position = r.f0;",
        "return c;",
    ] {
        assert!(text.contains(part), "{part} missing in {text}");
    }
    let _alone = drawing();
    if !kavach_runtime_gpu::available() {
        return;
    }
    let n = render::pipeline(&text).expect("a layout");
    let square = words(&[
        -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
    ]);
    let pixels = render::picture(vec![one(n, &square, &[], &[])], (8, 8)).expect("drawn");
    assert_eq!(
        pixels[4 * 8 + 4],
        [255, 0, 0, 255],
        "the middle, inside the half square"
    );
    assert_eq!(pixels[0], [0, 0, 0, 255], "a corner, outside it");
}

/// A `map` over `Vec3<Float32>`s is a kernel with WGSL's `vec3<f32>`, and
/// `std.vec`'s methods as WGSL's own (spec 14.28).
#[test]
fn a_map_over_vectors_uses_wgsls_vectors() {
    let k = crate::tests::kernel_of(
        "use std.vec

gpu fn size(v: Vec3<Float32>) -> Float32 {
    v.length() + v.dot(Vec3(x: 1.0, y: 0.0, z: 0.0))
}

fn main() {
    let vs: [Vec3<Float32>] = [Vec3(x: 3.0, y: 4.0, z: 0.0)]
    print(vs.map(v => size(v)))
}
",
    );
    for part in ["vec3<f32>", "length(", "dot("] {
        assert!(k.contains(part), "{part} missing in {k}");
    }
}

/// A `Mat3<Float32>` in a `gpu fn` is WGSL's `mat3x3<f32>`, its `@` and
/// `determinant` the GPU's own (spec 14.28).
#[test]
fn a_mat3_is_wgsls_mat3x3() {
    let k = crate::tests::kernel_of(
        "use std.vec

gpu fn turned(v: Vec3<Float32>) -> Float32 {
    const m = Mat3.scaling(Vec3(x: 2.0, y: 2.0, z: 2.0))
    (m @ v).length() + m.determinant()
}

fn main() {
    let vs: [Vec3<Float32>] = [Vec3(x: 3.0, y: 4.0, z: 0.0)]
    print(vs.map(v => turned(v)))
}
",
    );
    for part in ["mat3x3<f32>", "determinant(", "length("] {
        assert!(k.contains(part), "{part} missing in {k}");
    }
}
