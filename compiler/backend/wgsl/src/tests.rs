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
