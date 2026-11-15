//! Kernels written from whole programs.

use kavach_kir::{ExprKind, RtOp, walk};

/// The kernel text of the `map` in `src` (empty when the CPU runs it).
fn kernel_of(src: &str) -> String {
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    let mut program = kavach_kir::lower(&c).expect("lowered");
    crate::attach(&mut program);
    let mut text = String::new();
    for f in &program.functions {
        walk::any_in_block(&f.body, &mut |e| {
            if let ExprKind::Rt(RtOp::GpuMap, args) = &e.kind
                && let ExprKind::Str(s) = &args[2].kind
            {
                text = s.clone();
            }
            false
        });
    }
    text
}

/// The work in the first line of the kernel of the `map` in `src`.
fn work_of(src: &str) -> u64 {
    let text = kernel_of(src);
    let first = text.lines().next().unwrap_or_default();
    first
        .strip_prefix("// work ")
        .expect(first)
        .parse()
        .unwrap()
}

#[test]
fn a_loop_counts_once_per_round() {
    let map = |body: &str| {
        format!(
            "gpu fn f(x: Float32) -> Float32 {{\n{body}\n}}\n\nfn main() {{\n    let xs: [Float32] = [1.0]\n    print(xs.map(f))\n}}\n"
        )
    };
    let light = work_of(&map("    x * 2.0"));
    let looped = work_of(&map(
        "    let v = x\n    for _ in 0..1000 {\n        v = v * 0.5 + 1.0\n    }\n    v",
    ));
    assert!(light < 20, "{light}");
    assert!((3000..10_000).contains(&looped), "{looped}");
}

#[test]
fn a_record_item_comes_in_field_by_field() {
    let text = kernel_of(
        "type P {\n    x: Float32\n    v: Float32\n    on: Bool\n}\n\ngpu fn step(x: Float32, v: Float32) -> Float32 {\n    x + v\n}\n\nfn main() {\n    const ps = [P(x: 1.0, v: 2.0, on: true)]\n    print(ps.map(p => step(p.x, p.v)))\n}\n",
    );
    assert!(text.contains("items[i * 3u + 0u]"), "{text}");
    assert!(text.contains("items[i * 3u + 2u]"), "{text}");
}

#[test]
fn chained_maps_become_one_kernel() {
    // `xs.map(f).map(g)`: one `GpuMap`, whose kernel calls the second
    // function on the first's result, with both closures' captures.
    let src = "gpu fn scale(x: Float32, k: Float32) -> Float32 {\n    x * k\n}\n\ngpu fn shift(x: Float32, d: Float32) -> Float32 {\n    x + d\n}\n\nfn main() {\n    let xs: [Float32] = [1.0]\n    const k: Float32 = 2.0\n    const d: Float32 = 0.5\n    print(xs.map(x => scale(x, k)).map(y => shift(y, d)))\n}\n";
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    let mut program = kavach_kir::lower(&c).expect("lowered");
    crate::attach(&mut program);
    let mut maps = 0;
    for f in &program.functions {
        walk::any_in_block(&f.body, &mut |e| {
            if let ExprKind::Rt(RtOp::GpuMap, _) = &e.kind {
                maps += 1;
            }
            false
        });
    }
    assert_eq!(maps, 1, "two maps fused into one");
    let text = kernel_of(src);
    assert!(
        text.contains("cap1 = ") && text.contains("cap2 = "),
        "{text}"
    );
    let call = text
        .lines()
        .find(|l| l.contains("out[i] ="))
        .unwrap_or_default();
    // The second function's call holds the first's: `f2(f1(item))`.
    assert!(call.contains("f2(f1("), "{call}");
}

/// A record of four 32-bit fields comes in and goes out as one `vec4`,
/// which the GPU reads and writes faster than four words apart.
#[test]
fn four_fields_go_as_one_vec4() {
    let text = kernel_of(
        "type P {\n    x: Float32\n    y: Float32\n    vx: Float32\n    vy: Float32\n}\n\ngpu fn step(p: P) -> P {\n    P(x: p.x + p.vx, y: p.y + p.vy, vx: p.vx, vy: p.vy)\n}\n\nfn main() {\n    const ps = [P(x: 1.0, y: 2.0, vx: 0.5, vy: 0.25)]\n    print(ps.map(step)[0].x)\n}\n",
    );
    assert!(text.contains("items: array<vec4<u32>>;"), "{text}");
    assert!(text.contains("out: array<vec4<u32>>;"), "{text}");
    assert!(text.contains("items[i].w"), "{text}");
    assert!(text.contains("out[i] = vec4<u32>("), "{text}");
}

#[test]
fn a_known_divisor_has_no_guard() {
    let kernel = |body: &str| {
        kernel_of(&format!(
            "gpu fn work(x: Int32) -> Int32 {{\n{body}\n}}\nfn main() {{\n    let xs: [Int32] = []\n    for i in 0..10_000_000 {{ xs.push(Int32(i % 1000)) }}\n    print(xs.map(x => work(x)).len())\n}}\n"
        ))
    };
    let fixed = kernel(
        "    let v = x % 100\n    for _ in 0..200 {\n        v = (v * 3 + 1) % 100\n    }\n    v / 7",
    );
    let f1 = &fixed[fixed.find("fn f1(").expect("the kernel's function")..];
    assert!(!f1.contains("rem_i") && !f1.contains("div_i"), "{f1}");
    assert!(
        !f1.contains("halted"),
        "nothing here can stop the item: {f1}"
    );
    for (body, helper) in [
        ("x % (x + 1)", "rem_i"),
        ("x / (x - 2)", "div_i"),
        ("x % -1", "rem_i"),
        ("x / 0", "div_i"),
    ] {
        let k = kernel(&format!("    {body}"));
        let f1 = &k[k.find("fn f1(").expect("the kernel's function")..];
        assert!(f1.contains(helper), "{body}: {f1}");
    }
}

#[test]
fn only_loops_that_surely_end_are_marked() {
    let kernel = |body: &str| {
        kernel_of(&format!(
            "gpu fn work(x: Int32) -> Int32 {{\n    let v = x\n{body}\n    v\n}}\nfn main() {{\n    let xs: [Int32] = []\n    for i in 0..10_000_000 {{ xs.push(Int32(i % 1000)) }}\n    print(xs.map(x => work(x)).len())\n}}\n"
        ))
    };
    let marked = |k: &str| k.lines().take(6).any(|l| l == crate::LOOPS_END);
    for body in [
        "",
        "    for _ in 0..200 {\n        v = (v * 3 + 1) % 100\n    }",
    ] {
        let k = kernel(body);
        assert!(!k.is_empty() && marked(&k), "{body}: {k}");
    }
    for body in [
        "    while v > 3 {\n        v -= 2\n    }",
        "    for _ in 0..=200 {\n        v = (v * 3 + 1) % 100\n    }",
        "    for _ in 0..4 {\n        while v > 3 {\n            v -= 2\n        }\n    }",
    ] {
        let k = kernel(body);
        assert!(!k.is_empty() && !marked(&k), "{body}: {k}");
    }
}

/// The shaders of the `gpu.pipeline` in `src`.
fn pipeline_of(src: &str) -> String {
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    assert!(!c.has_errors(), "{}", c.render());
    let mut program = kavach_kir::lower(&c).expect("lowered");
    crate::attach(&mut program);
    let mut text = String::new();
    for f in &program.functions {
        walk::any_in_block(&f.body, &mut |e| {
            if let ExprKind::Rt(RtOp::DrawPipeline, args) = &e.kind
                && let ExprKind::Str(s) = &args[2].kind
            {
                text = s.clone();
            }
            false
        });
    }
    text
}

/// The spec's example (15.3): a vertex of two `Float32`s, an instance of
/// three, a shared value of one word, and `heat` blended to the pixel.
pub(crate) const SWARM: &str = "use std.gpu

type Corner {
    x: Float32
    y: Float32
}

type Dot {
    x: Float32
    y: Float32
    heat: Float32
}

type Camera {
    zoom: Float32
}

type Lit {
    position: (Float32, Float32, Float32, Float32)
    heat: Float32
}

gpu vertex fn place(c: Corner, d: Dot, cam: Camera) -> Lit {
    Lit(position: ((c.x + d.x) * cam.zoom, (c.y + d.y) * cam.zoom, 0.0, 1.0), heat: d.heat)
}

gpu fragment fn glow(l: Lit) -> (Float32, Float32, Float32, Float32) {
    (l.heat, 0.2, 1.0 - l.heat, 1.0)
}

fn main() {
    const _ = gpu.pipeline(place, glow)
}
";

#[test]
fn a_pipeline_is_one_module_with_its_layout_first() {
    let text = pipeline_of(SWARM);
    assert!(text.starts_with("// draw f f | f f f | 1\n"), "{text}");
    for part in [
        "@vertex\nfn vs_main(a: VIn) -> VOut",
        "@fragment\nfn fs_main(i: VOut)",
        "@builtin(position) position: vec4<f32>",
    ] {
        assert!(text.contains(part), "{part} missing in {text}");
    }
}

/// The swarm example's shaders, drawn off-screen: each dot's triangle
/// takes its color from its `heat`, and what no triangle covers stays
/// black. Skipped on a machine with no GPU.
#[test]
fn a_pipeline_draws_what_its_gpu_fns_say() {
    if !kavach_runtime_gpu::available() {
        return;
    }
    use kavach_runtime_gpu::render::Items::Words;
    let n = kavach_runtime_gpu::render::pipeline(&pipeline_of(SWARM)).expect("a layout");
    let words = |fs: &[f32]| fs.iter().map(|f| f.to_bits()).collect::<Vec<u32>>();
    let corners = words(&[-0.2, -0.2, 0.2, -0.2, 0.0, 0.2]);
    // A hot dot in the middle, a cold one up and to the right.
    let dots = words(&[0.0, 0.0, 1.0, 0.5, 0.5, 0.0]);
    let camera = words(&[1.0]);
    let pixels =
        kavach_runtime_gpu::render::picture(n, (Words(&corners), Words(&dots), &camera), (32, 32))
            .expect("drawn");
    let at = |x: usize, y: usize| pixels[y * 32 + x];
    assert_eq!(at(16, 16), [255, 51, 0, 255], "the hot dot");
    assert_eq!(at(24, 8), [0, 51, 255, 255], "the cold dot");
    assert_eq!(at(1, 1), [0, 0, 0, 255], "nothing drawn there");
}

/// A pipeline made once something has been drawn is compiled at once, not
/// at its first draw, so a frame never waits for it (decision 0106).
#[test]
fn a_pipeline_compiles_before_its_first_draw() {
    if !kavach_runtime_gpu::available() {
        return;
    }
    use kavach_runtime_gpu::render::{self, Items::Words};
    let wgsl = pipeline_of(SWARM);
    let first = render::pipeline(&wgsl).expect("a layout");
    let words = |fs: &[f32]| fs.iter().map(|f| f.to_bits()).collect::<Vec<u32>>();
    let corners = words(&[-0.2, -0.2, 0.2, -0.2, 0.0, 0.2]);
    let (dot, camera) = (words(&[0.0, 0.0, 1.0]), words(&[1.0]));
    render::picture(first, (Words(&corners), Words(&dot), &camera), (8, 8)).expect("drawn");
    let second = render::pipeline(&format!("{wgsl}// another\n")).expect("a layout");
    assert!(render::is_made(second), "compiled when it was made");
}
