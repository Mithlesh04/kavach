// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Slices of lists in GPU memory, made there (spec 15.4), in native
//! code: forced on with `KAVACH_GPU=on`, against the interpreter and
//! `KAVACH_GPU=cpu`. Without a GPU the runtime uses the CPU, so these
//! pass either way.

use std::process::Command;

/// Lists a loop of `map`s leaves in GPU memory, of one word an item and
/// of records, sliced: kept, read, mapped, summed, sliced again.
const SLICED: &str = "type Ball {
    y: Float32
    vy: Float32
}

gpu fn step(x: Int32) -> Int32 {
    x + 1
}

gpu fn fall(b: Ball) -> Ball {
    Ball(y: b.y + b.vy, vy: b.vy - 0.5)
}

fn main() {
    let xs: [Int32] = []
    let balls: [Ball] = []
    for i in 0..2000 {
        xs.push(Int32(i))
        balls.push(Ball(y: Float32(i), vy: 2.0))
    }
    for _ in 0..4 {
        xs = xs.map(step)
        balls = balls.map(fall)
    }
    let firsts = xs[0:1000]
    firsts = firsts.map(step)
    print(`{firsts[0]} {firsts[#1]} {firsts.len()}`)
    print(xs[::-500])
    print(xs[#3:])
    print(xs[3000:])
    print(xs[1::2].map(x => step(x) * 2).sum(x => Int(x)))
    print(xs[10:20:3][::-1])
    const back = balls[::-1]
    print(back[0])
    print(balls[#2:].map(fall))
}
";

/// A step of 0, worked out while running, on a list in GPU memory.
const STEP_ZERO: &str = "gpu fn step(x: Int32) -> Int32 {
    x + 1
}

fn main() {
    let xs: [Int32] = [1, 2, 3]
    xs = xs.map(step)
    const k = xs.len() - 3
    print(xs[::k])
}
";

/// Runs `src`, natively or in the interpreter, with `KAVACH_GPU=mode`:
/// its output and errors.
fn run(src: &str, native: bool, mode: &str) -> (String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kavach-gpu-slices-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    cmd.arg(dir.join("main.kav"));
    cmd.env_remove("KAVACH_FAKE_GPUS")
        .env("KAVACH_GPU", mode)
        .env("KAVACH_CHECK_LEAKS", "1");
    let out = cmd.output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (text(&out.stdout), text(&out.stderr))
}

#[test]
fn slices_on_the_gpu_give_the_cpus_items() {
    let (want, err) = run(SLICED, false, "cpu");
    assert_eq!(
        want,
        "5 1004 1000
[2003, 1503, 1003, 503]
[2001, 2002, 2003]
[]
2010000
[23, 20, 17, 14]
Ball(y: 2004.0, vy: 0.0)
[Ball(y: 2003.0, vy: -0.5), Ball(y: 2004.0, vy: -0.5)]
",
        "{err}"
    );
    for mode in ["on", "check", "cpu"] {
        let (got, err) = run(SLICED, true, mode);
        assert_eq!(got, want, "KAVACH_GPU={mode}: {err}");
        assert!(!err.contains("not freed"), "KAVACH_GPU={mode}: {err}");
    }
}

#[test]
fn a_step_of_zero_panics_as_on_the_cpu() {
    let (_, cpu) = run(STEP_ZERO, false, "cpu");
    let (_, gpu) = run(STEP_ZERO, true, "on");
    let line = |err: &str| {
        let at = err.lines().find(|l| l.contains("main.kav:"));
        at.map(|l| l.rsplit(['/', '\\']).next().unwrap_or(l).to_string())
    };
    assert!(cpu.contains("the step of a slice can't be 0"), "{cpu}");
    assert!(gpu.contains("the step of a slice can't be 0"), "{gpu}");
    assert_eq!(line(&gpu), line(&cpu), "{cpu}\n{gpu}");
}
