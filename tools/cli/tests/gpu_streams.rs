//! Sums a loop doesn't wait for (spec 15.2, decision 0067), in native code:
//! the same answers and the same panics as the CPU, wherever they run; and
//! a kernel's multiply at the limits of its check. Without a GPU the
//! runtime uses the CPU, so these pass either way.

use std::process::Command;

fn run(src: &str, name: &str, native: bool, mode: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("kavach-streams-{name}-{}", std::process::id()));
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

/// The panic message and line, without the file's path.
fn panic_of(err: &str) -> Vec<String> {
    err.lines()
        .filter(|l| l.contains("panic") || l.contains("  at "))
        .map(|l| l.rsplit(['/', '\\']).next().unwrap_or(l).to_string())
        .collect()
}

/// `src`'s output on the CPU, and the same from native code whichever way
/// the GPU is set.
fn same_everywhere(src: &str, name: &str) -> String {
    let (cpu, err) = run(src, &format!("{name}-i"), false, "off");
    for mode in ["on", "off", "check", ""] {
        let (got, native) = run(src, &format!("{name}-{mode}"), true, mode);
        assert_eq!(got, cpu, "KAVACH_GPU={mode}: {native}");
        assert_eq!(panic_of(&native), panic_of(&err), "KAVACH_GPU={mode}");
        assert!(!native.contains("not freed"), "{native}");
    }
    cpu
}

/// More rounds than can be under way at once, of each reduction, a filter
/// before one, and a sum kept in a `const`.
const ROUNDS: &str = "gpu fn step(x: Int32) -> Int32 {
    (x * 7 + 3) % 1001 - 500
}

gpu fn half(x: UInt32) -> UInt32 {
    x / 2 + 1
}

fn main() {
    let total = 0
    let widest = 0
    let lowest: Int32? = none
    let kept = 0
    let unsigned: UInt32 = 0
    for round in 0..8 {
        let xs: [Int32] = []
        let us: [UInt32] = []
        for i in 0..1000 {
            xs.push(Int32(i + round * 100))
            us.push(UInt32(i * 3))
        }
        total += Int(xs.map(step).sum(x => x))
        const biggest = xs.map(step).max() ?? 0
        widest = widest.max(Int(biggest))
        lowest = xs.map(step).min()
        const s = xs.map(step).filter(x => x > 0).sum(x => x + 1)
        kept += Int(s)
        unsigned += us.map(half).sum(u => u) % 1000
    }
    print(\"{total} {widest} {lowest} {kept} {unsigned}\")
}
";

#[test]
fn sums_in_a_loop_give_the_cpus_answers() {
    assert_eq!(
        same_everywhere(ROUNDS, "rounds"),
        same_everywhere(ROUNDS, "rounds-2")
    );
}

/// A sum that overflows in the third round, as the CPU adds it: the rounds
/// before it are added, and the panic names the same line.
const OVERFLOWS: &str = "gpu fn big(x: Int32) -> Int32 {
    x * 1000000
}

fn main() {
    let total = 0
    for round in 0..6 {
        let xs: [Int32] = []
        for i in 0..3000 {
            xs.push(Int32(if round == 2 { i % 3 } else { i % 2 - 1 }))
        }
        total += Int(xs.map(big).sum(x => x))
    }
    print(total)
}
";

#[test]
fn a_sum_that_overflows_panics_as_the_cpu_adds_it() {
    let out = same_everywhere(OVERFLOWS, "overflows");
    assert_eq!(out, "");
}

/// The statement's own check, on the running total: the third round's.
const TOTAL_OVERFLOWS: &str = "gpu fn one(x: Int32) -> Int32 {
    x * 0 + 1
}

fn main() {
    let total: Int32 = 2147483647 - 5000
    for round in 0..6 {
        let xs: [Int32] = []
        for i in 0..2000 {
            xs.push(Int32(i + round))
        }
        total += xs.map(one).sum(x => x)
    }
    print(total)
}
";

#[test]
fn the_statement_keeps_its_own_checks_in_order() {
    let (_, cpu) = run(TOTAL_OVERFLOWS, "total-i", false, "off");
    assert!(cpu.contains("integer overflow: 2147482647 + 2000"), "{cpu}");
    same_everywhere(TOTAL_OVERFLOWS, "total");
}

/// A `gpu fn` that panics in a round: the same message and line, and
/// nothing after the loop runs.
const STEP_PANICS: &str = "gpu fn step(x: Int32) -> Int32 {
    100 / (x - 5000)
}

fn main() {
    let total = 0
    for round in 0..6 {
        let xs: [Int32] = []
        for i in 0..2000 {
            xs.push(Int32(i + round * 1500))
        }
        total += Int(xs.map(step).sum(x => x))
    }
    print(total)
}
";

#[test]
fn a_panic_in_a_round_is_the_cpus() {
    let (_, cpu) = run(STEP_PANICS, "step-i", false, "off");
    assert!(cpu.contains("division by zero"), "{cpu}");
    let (out, gpu) = run(STEP_PANICS, "step-on", true, "on");
    assert_eq!(out, "", "{gpu}");
    assert_eq!(panic_of(&gpu), panic_of(&cpu));
}

/// Leaving the function in the middle: what is under way still counts, and
/// nothing is left held.
const LEAVES: &str = "gpu fn step(x: Int32) -> Int32 {
    x + 1
}

fn add_up(limit: Int) -> Int ! Error {
    let total = 0
    for round in 0..8 {
        let xs: [Int32] = []
        for i in 0..1000 {
            xs.push(Int32(i))
        }
        total += Int(xs.map(step).sum(x => x))
        if round == limit {
            fail Error(\"stopped at {round}\")
        }
    }
    total
}

fn break_out() -> Int {
    let total = 0
    for round in 0..8 {
        const xs: [Int32] = [1, 2, 3]
        total += Int(xs.map(step).sum(x => x))
        if round == 3 {
            break
        }
    }
    total
}

fn main() {
    print(add_up(100) catch e => -1)
    print(add_up(2) catch e => -2)
    print(break_out())
}
";

#[test]
fn leaving_a_loop_early_is_the_same() {
    let out = same_everywhere(LEAVES, "leaves");
    assert_eq!(out, "4004000\n-2\n36\n");
}

/// Two tasks in loops of their own don't share a queue.
const TASKS: &str = "use std.time

gpu fn step(x: Int32) -> Int32 {
    x + 1
}

fn add_up(base: Int32) -> Int {
    let total = 0
    for round in 0..6 {
        let xs: [Int32] = []
        for i in 0..500 {
            xs.push(base + Int32(i))
        }
        total += Int(xs.map(step).sum(x => x))
        time.sleep(1.ms)
    }
    total
}

fn main() {
    const a = spawn add_up(0)
    const b = spawn add_up(1000)
    print(\"{a.wait() catch e => 0} {b.wait() catch e => 0}\")
}
";

#[test]
fn tasks_keep_their_own_sums() {
    same_everywhere(TASKS, "tasks");
}

/// `kavach check` says which loops wait for the GPU, and why (spec 15.2).
fn notes_of(src: &str, name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kavach-streams-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    std::fs::write(&file, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["check"])
        .arg(&file)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// `{body}` in a loop over `rounds`.
fn looping(body: &str) -> String {
    format!(
        "gpu fn step(x: Int32) -> Int32 {{\n    x + 1\n}}\n\nfn main() {{\n    const rounds: [[Int32]] = [[1, 2]]\n    let total = 0\n    for xs in rounds {{\n{body}\n    }}\n    print(total)\n}}\n"
    )
}

#[test]
fn check_says_which_loops_wait_and_why() {
    let cases = [
        (
            "        total += Int(xs.map(step).sum(x => x))\n        print(total)",
            "main.kav:9: note: this loop waits for the GPU each round: the loop also reads what it adds up to",
        ),
        (
            "        print(xs.map(step).sum(x => x))",
            "main.kav:9: note: this loop waits for the GPU each round: the round uses the sum at once",
        ),
        (
            "        print(xs.map(step))",
            "main.kav:9: note: this loop waits for the GPU each round: the CPU reads the result each round",
        ),
    ];
    for (i, (body, note)) in cases.iter().enumerate() {
        let said = notes_of(&looping(body), &format!("note-{i}"));
        assert!(said.contains(note), "{said}");
    }
    let quiet = "        total += Int(xs.map(step).sum(x => x))";
    let said = notes_of(&looping(quiet), "note-quiet");
    assert!(!said.contains("note:"), "{said}");
}

/// `a * b` of each pair, of type `t`, in a kernel.
fn multiplying(t: &str, pairs: &[(i64, i64)]) -> String {
    let list: Vec<String> = pairs
        .iter()
        .map(|(a, b)| format!("({t}({a}), {t}({b}))"))
        .collect();
    format!(
        "gpu fn times(a: {t}, b: {t}) -> {t} {{
    a * b
}}

fn main() {{
    const pairs: [({t}, {t})] = [{}]
    print(pairs.map(p => times(p.0, p.1)))
}}
",
        list.join(", ")
    )
}

/// Each side of where a kernel's multiply skips its division (16 bits), and
/// of the type's limits: the CPU's product, or its panic.
#[test]
fn a_kernels_multiply_is_the_cpus_at_the_limits() {
    let i32_fit = [
        (32767, 32767),
        (-32768, -32768),
        (-32768, 32767),
        (32768, 32767),
        (-32769, 1),
        (46340, 46340),
        (65536, 32767),
        (65536, -32768),
        (2147483647, 1),
        (-2147483648, 1),
        (0, -2147483648),
        (2147483647, -1),
    ];
    same_everywhere(&multiplying("Int32", &i32_fit), "mul-i32");
    let u32_fit = [
        (65535, 65535),
        (65536, 65535),
        (4294967295, 1),
        (0, 4294967295),
        (2147483647, 2),
    ];
    same_everywhere(&multiplying("UInt32", &u32_fit), "mul-u32");
    let too_big: [(&str, (i64, i64)); 6] = [
        ("Int32", (46341, 46341)),
        ("Int32", (65536, 32768)),
        ("Int32", (65536, -32769)),
        ("Int32", (-2147483648, -1)),
        ("UInt32", (65536, 65536)),
        ("UInt32", (4294967295, 2)),
    ];
    for (i, (t, pair)) in too_big.iter().enumerate() {
        let src = multiplying(t, &[*pair]);
        let (_, err) = run(&src, &format!("mul-big-{i}-i"), false, "off");
        assert!(err.contains("integer overflow"), "{t} {pair:?}: {err}");
        same_everywhere(&src, &format!("mul-big-{i}"));
    }
}
