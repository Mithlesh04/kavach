//! `map` on the GPU (spec 15.2), in native code: forced on with
//! `KAVACH_GPU=on`, and compared with the CPU by `KAVACH_GPU=check`.
//! Without a GPU the runtime uses the CPU, so these pass either way.

use std::process::Command;

/// Exact on any machine: whole numbers, and floats whose every step is
/// exact, so a GPU that fuses multiply and add gives the same bits.
const EXACT: &str = "gpu fn step(x: Float32, k: Float32) -> Float32 {
    x * 2.0 + k
}

gpu fn mix(n: Int32, m: Int32) -> Int32 {
    let s = 0
    for i in 0..n {
        s = s + i * m
    }
    s
}

fn main() {
    let xs: [Float32] = []
    let ns: [Int32] = []
    for i in 0..1000 {
        xs.push(Float32(i))
        ns.push(Int32(i % 50))
    }
    const k: Float32 = 0.5
    const ys = xs.map(x => step(x, k))
    print(\"{ys[0]} {ys[999]} {ys.len()}\")
    const ms = ns.map(n => mix(n, 3))
    print(\"{ms[49]} {ms[10]}\")
}
";

/// Items that are records and tuples, read field by field.
const FIELDS: &str = "use std.collections

type Particle {
    x: Float32
    v: Float32
    alive: Bool
}

gpu fn step(x: Float32, v: Float32, alive: Bool) -> Float32 {
    if alive {
        x + v * 0.5
    } else {
        x
    }
}

gpu fn madd(a: Int32, b: Int32) -> Int32 {
    a * 3 + b
}

fn main() {
    let ps: [Particle] = []
    let a: [Int32] = []
    for i in 0..1000 {
        ps.push(Particle(x: Float32(i), v: 2.0, alive: i % 3 != 0))
        a.push(Int32(i))
    }
    const moved = ps.map(p => step(p.x, p.v, p.alive))
    print(\"{moved[0]} {moved[1]} {moved[999]}\")
    const m = collections.zip(a, a.reverse()).map(t => madd(t.0, t.1))
    print(\"{m[0]} {m[999]} {m.len()}\")
}
";

/// Overflow inside a kernel: the same panic as on the CPU.
const OVERFLOW: &str = "gpu fn square(n: Int32) -> Int32 {
    n * n
}

fn main() {
    let ns: [Int32] = []
    for i in 0..1000 {
        ns.push(Int32(i))
    }
    ns.push(100000)
    print(ns.map(square).len())
}
";

/// Overflow in a `map` whose list stays on the GPU, which doesn't wait
/// (decision 0061): reported when the list is read, or at the end.
const LATE: &str = "gpu fn square(n: Int32) -> Int32 {
    n * n
}

fn main() {
    let ns: [Int32] = []
    for i in 0..1000 {
        ns.push(Int32(i % 7))
    }
    ns.push(100000)
    for _ in 0..3 {
        ns = ns.map(square)
    }
    print(\"looped\")
    READ
}
";

/// Rounding that a fused multiply-add can change: `check` must accept it.
const ROUNDED: &str = "gpu fn glow(x: Float32, gain: Float32) -> Float32 {
    let v = x
    for _ in 0..100 {
        v = v * 0.99 + gain
    }
    v
}

fn main() {
    let xs: [Float32] = []
    for i in 0..5000 {
        xs.push(Float32(i % 7) * 0.25)
    }
    const g: Float32 = 0.5
    print(xs.map(x => glow(x, g))[6])
}
";

/// Lists the GPU keeps in its memory (spec 15.2), changed in place, and
/// mapped again: each `map` must see the list as it is now.
const KEPT: &str = "gpu fn twice(x: Int32) -> Int32 {
    x * 2
}

gpu fn plus(x: Int32, k: Int32) -> Int32 {
    x + k
}

fn main() {
    let xs: [Int32] = [1, 2, 3, 4]
    print(xs.map(twice))
    xs[0] = 100
    xs.push(7)
    print(xs.map(twice))
    const ys = xs.map(twice)
    print(ys.map(k => plus(k, 1)))
    print(ys)
    let v: [Int32] = [1, 1, 1]
    for i in 0..5 {
        v = v.map(k => plus(k, Int32(i)))
    }
    let w = v
    w[1] = -1
    print(\"{w.map(twice)} {v.map(twice)}\")
}
";

/// Results that stay on the GPU until the CPU reads them (spec 15.2): each
/// way of reading one must see its items, and so must a list let go of
/// to make room (more than four).
const STAYS: &str = "gpu fn step(x: Int32) -> Int32 {
    (x * 3 + 1) % 1009
}

fn total(xs: [Int32]) -> Int {
    let t = 0
    for x in xs {
        t += Int(x)
    }
    t
}

fn main() {
    let xs: [Int32] = []
    for i in 0..300 {
        xs.push(Int32(i))
    }
    for _ in 0..3 {
        xs = xs.map(step)
    }
    let sum = 0
    for i in 0..xs.len() {
        sum += Int(xs[i])
    }
    print(\"{sum} {total(xs)} {xs[299]}\")
    const a = xs.map(step)
    const b = a.map(step)
    const c = b.map(step)
    const d = c.map(step)
    const e = d.map(step)
    const f = e.map(step)
    const at = [a, b, c, d, e, f].map(l => l[5])
    print(at)
    const first = (n: Int) => Int(a[n])
    print(first(1))
    let sorted = f
    sorted[0] = 0
    print(\"{sorted[0]} {f[0] == sorted[0]}\")
}
";

/// `sum`, `min` and `max` at the end of a GPU pipeline (spec 15.2), on
/// both 32-bit whole number types, and on an empty list.
const REDUCED: &str = "gpu fn step(x: Int32) -> Int32 {
    (x * 7 + 3) % 1001 - 500
}

gpu fn half(x: UInt32) -> UInt32 {
    x / 2 + 1
}

fn main() {
    let xs: [Int32] = []
    let us: [UInt32] = []
    for i in 0..5000 {
        xs.push(Int32(i))
        us.push(UInt32(i * 3))
    }
    xs = xs.map(step)
    us = us.map(half)
    print(\"{xs.sum(x => x)} {xs.sum(x => step(x))} {xs.min()} {xs.max()}\")
    print(\"{us.sum(u => u)} {us.min()} {us.max()}\")
    let empty: [Int32] = []
    empty = empty.map(step)
    print(\"{empty.sum(x => x)} {empty.min()}\")
}
";

/// A sum that overflows partway, as the CPU adds it: the same panic on
/// the GPU, at the same line.
const SUM_OVERFLOWS: &str = "gpu fn big(x: Int32) -> Int32 {
    x * 1000000
}

fn main() {
    let xs: [Int32] = []
    for i in 0..3000 {
        xs.push(Int32(i % 3))
    }
    xs = xs.map(big)
    print(xs.sum(x => x))
}
";

/// `filter` in a GPU pipeline (spec 15.2): the kept items in order, mapped
/// and summed after it, a filter that keeps none, and one of floats.
const FILTERED: &str = "gpu fn step(x: Int32) -> Int32 {
    (x * 7 + 3) % 1001 - 500
}

fn main() {
    let xs: [Int32] = []
    for i in 0..20000 {
        xs.push(Int32(i))
    }
    xs = xs.map(step)
    let ys = xs.filter(x => x > 400)
    print(\"{ys.len()} {ys[0]} {ys[ys.len() - 1]} {ys.take(4)}\")
    ys = ys.map(step)
    print(ys.sum(y => y))
    let nothing = xs.filter(x => x > 1000)
    print(nothing.len())
    let fs: [Float32] = [1.5, -2.0, 3.25, 0.0]
    fs = fs.map(f => f * 2.0)
    print(fs.filter(f => f > 0.0))
}
";

/// A `gpu fn` that takes and gives records and tuples (spec 15.1): balls
/// falling for 40 frames, each frame's list staying on the GPU.
const RECORDS: &str = "type Ball {
    y: Float32
    vy: Float32
    bounces: Int32
    up: Bool
}

gpu fn fall(b: Ball) -> Ball {
    let vy = b.vy - 0.5
    let y = b.y + vy
    let bounces = b.bounces
    if y < 0.0 {
        vy = 0.0 - vy
        y = 0.0
        bounces = bounces + 1
    }
    Ball(y: y, vy: vy, bounces: bounces, up: vy > 0.0)
}

gpu fn spread(b: Ball) -> (Int32, Float32) {
    (b.bounces * 2, b.y + 1.0)
}

fn main() {
    let balls: [Ball] = []
    for i in 0..5000 {
        balls.push(Ball(y: Float32(i % 50), vy: 0.0, bounces: 0, up: false))
    }
    for _ in 0..40 {
        balls = balls.map(fall)
    }
    print(balls[0])
    print(balls[4999])
    print(balls.map(b => b.bounces).sum(n => Int(n)))
    const pairs = balls.map(spread)
    print(pairs[7])
}
";

fn run(src: &str, name: &str, native: bool, mode: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("kavach-gpu-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    std::fs::write(&file, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    let out = cmd
        .arg(&file)
        .env("KAVACH_GPU", mode)
        .env("KAVACH_CHECK_LEAKS", "1")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).to_string();
    (text(&out.stdout), text(&out.stderr))
}

#[test]
fn gpu_maps_give_the_cpus_answers() {
    let (cpu, _) = run(EXACT, "exact-i", false, "off");
    assert_eq!(cpu, "0.5 1998.5 1000\n3528 135\n");
    for mode in ["on", "check", "off"] {
        let (got, err) = run(EXACT, &format!("exact-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {err}");
    }
}

#[test]
fn overflow_in_a_kernel_panics_as_on_the_cpu() {
    let (_, err) = run(OVERFLOW, "overflow", true, "on");
    assert!(
        err.contains("panic: integer overflow: 100000 * 100000 doesn't fit in Int32"),
        "{err}"
    );
    // The line inside the `gpu fn`, as the interpreter says it, not the
    // `map`'s.
    assert!(err.contains(":2"), "{err}");
}

#[test]
fn overflow_in_a_map_that_did_not_wait_panics_later() {
    for (read, name) in [("print(ns[0])", "late-read"), ("", "late-end")] {
        let (out, err) = run(&LATE.replace("READ", read), name, true, "on");
        assert!(
            err.contains("panic: integer overflow: 100000 * 100000 doesn't fit in Int32"),
            "{name}: {err}"
        );
        assert!(err.contains(":2"), "{name}: {err}");
        assert!(
            !out.contains(
                "
0
"
            ),
            "{name}: {out}"
        );
    }
}

#[test]
fn check_accepts_rounding_differences() {
    let (out, err) = run(ROUNDED, "rounded", true, "check");
    let (cpu, _) = run(ROUNDED, "rounded-i", false, "off");
    assert_eq!(out, cpu, "{err}");
}

#[test]
fn kept_lists_change_like_any_list() {
    let (cpu, _) = run(KEPT, "kept-i", false, "off");
    assert_eq!(
        cpu,
        "[2, 4, 6, 8]
[200, 4, 6, 8, 14]
[201, 5, 7, 9, 15]
[200, 4, 6, 8, 14]
[22, -2, 22] [22, 22, 22]
"
    );
    let (got, err) = run(KEPT, "kept-on", true, "on");
    assert_eq!(got, cpu, "{err}");
    assert!(!err.contains("not freed"), "{err}");
}

#[test]
fn record_and_tuple_items_give_the_cpus_answers() {
    let (cpu, _) = run(FIELDS, "fields-i", false, "off");
    assert_eq!(cpu, "0.0 2.0 999.0\n999 2997 1000\n");
    for mode in ["on", "check"] {
        let (got, err) = run(FIELDS, &format!("fields-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {err}");
    }
}

#[test]
fn results_left_on_the_gpu_read_the_same_on_the_cpu() {
    let (cpu, _) = run(STAYS, "stays-i", false, "off");
    for mode in ["on", "check"] {
        let (got, err) = run(STAYS, &format!("stays-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {err}");
        assert!(!err.contains("not freed"), "{err}");
    }
}

#[test]
fn sums_and_extremes_on_the_gpu_give_the_cpus_answers() {
    let (cpu, _) = run(REDUCED, "reduced-i", false, "off");
    assert_eq!(
        cpu,
        "-2415 -2486890 -497 497
18750000 1 7499
0 none
"
    );
    for mode in ["on", ""] {
        let (got, err) = run(REDUCED, &format!("reduced-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {err}");
        assert!(!err.contains("not freed"), "{err}");
    }
    let (_, cpu) = run(SUM_OVERFLOWS, "sum-overflows-i", false, "off");
    let (_, gpu) = run(SUM_OVERFLOWS, "sum-overflows-on", true, "on");
    assert!(cpu.contains("integer overflow"), "{cpu}");
    assert_eq!(panic_of(&gpu), panic_of(&cpu));
}

/// The panic message and line, without the file's path.
fn panic_of(err: &str) -> Vec<String> {
    err.lines()
        .filter(|l| l.contains("panic") || l.contains("  at "))
        .map(|l| l.rsplit(['/', '\\']).next().unwrap_or(l).to_string())
        .collect()
}

#[test]
fn filters_on_the_gpu_keep_the_cpus_items_in_order() {
    let (cpu, _) = run(FILTERED, "filtered-i", false, "off");
    assert_eq!(
        cpu,
        "1946 406 497 [406, 413, 420, 427]\n-104111\n0\n[3.0, 6.5]\n"
    );
    for mode in ["on", ""] {
        let (got, err) = run(FILTERED, &format!("filtered-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {err}");
        assert!(!err.contains("not freed"), "{err}");
    }
}

#[test]
fn records_on_the_gpu_give_the_cpus_answers() {
    let (cpu, _) = run(RECORDS, "records-i", false, "off");
    assert_eq!(
        cpu,
        "Ball(y: 0.0, vy: 0.0, bounces: 20, up: false)
Ball(y: 6.5, vy: -6.0, bounces: 1, up: false)
14900
(8, 6.0)
"
    );
    for mode in ["on", "check", ""] {
        let (got, err) = run(RECORDS, &format!("records-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {err}");
        assert!(!err.contains("not freed"), "{err}");
    }
}
