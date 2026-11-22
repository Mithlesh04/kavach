// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! One big `map` split across GPUs (spec 15.6, decision 0144): the same
//! values, in the same order, as the interpreter's CPU gives, with exact
//! sums and the CPU's panic. `KAVACH_FAKE_GPUS` lists two devices that
//! open real GPUs of this machine (a fake entry opens the GPU of its
//! name), and `KAVACH_GPU_SPLIT=on` splits small lists; `KAVACH_PAR_LOG`
//! shows the split. Skipped where the machine has no GPU to open.

use std::process::Command;

/// Runs `src` with `env`, natively or not: its output, errors and code.
fn run(src: &str, native: bool, env: &[(&str, &str)]) -> (String, String, i32) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kavach-gpu-split-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    cmd.arg(dir.join("main.kav")).current_dir(&dir);
    for k in [
        "KAVACH_GPU",
        "KAVACH_FAKE_GPUS",
        "KAVACH_GPU_SPLIT",
        "KAVACH_GPU_WORDS",
    ] {
        cmd.env_remove(k);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (
        text(&out.stdout),
        text(&out.stderr),
        out.status.code().unwrap_or(-1),
    )
}

/// The machine's GPUs that open, with their kinds, as `system.gpus()`
/// names them: software ones too (WARP, llvmpipe), as a fake entry may
/// call any GPU discrete.
fn real_gpus() -> Vec<(String, String)> {
    let src = "use std.system\n\nfn main() {\n    for g in system.gpus() {\n        print(`{g.name}|{g.backend}`)\n    }\n}\n";
    let (out, _, _) = run(src, false, &[]);
    (out.lines())
        .filter_map(|l| l.split_once('|'))
        .map(|(n, b)| (n.to_string(), b.to_string()))
        .collect()
}

/// Two fake discrete devices: the machine's first GPU, and its last (the
/// same one when it has only one, opened twice).
fn two_devices() -> Option<String> {
    let real = real_gpus();
    let (first, last) = (real.first()?, real.last()?);
    let entry = |(name, backend): &(String, String)| format!("{name}:0x10de:discrete:{backend}");
    Some(format!("{};{}", entry(first), entry(last)))
}

const SPLIT: &str = "type Ball {
    y: Float32
    vy: Float32
}

gpu fn step(x: Float32) -> Float32 {
    let v = x
    for _ in 0..3 {
        v = v * 0.75 + 1.0
    }
    v
}

gpu fn twice(n: Int32) -> Int32 {
    n * 2 + 1
}

gpu fn fall(b: Ball) -> Ball {
    const vy = b.vy - 9.8 * 0.016
    Ball(y: b.y + vy * 0.016, vy: vy)
}

fn main() {
    let xs: [Float32] = []
    let ns: [Int32] = []
    let balls: [Ball] = []
    for i in 0..5000 {
        xs.push(Float32(i % 97) * 0.25)
        ns.push(Int32(i))
        balls.push(Ball(y: Float32(i), vy: 0.0))
    }
    const ys = xs.map(step)
    const ms = ns.map(twice)
    const fallen = balls.map(fall)
    print(`{ys[0]} {ys[2047]} {ys[2048]} {ys[4999]} {ys.len()}`)
    print(`{ms[0]} {ms[2048]} {ms[4999]} {ms.sum(x => x)}`)
    print(`{fallen[2048].y} {fallen[4999].vy} {fallen.len()}`)
    print(ys.sum(x => x))
    let wrong = 0
    for i in 0..5000 {
        if ms[i] != Int32(i) * 2 + 1 {
            wrong += 1
        }
    }
    print(`{wrong} out of order`)
}
";

#[test]
fn a_split_map_gives_the_cpus_values_in_order() {
    let Some(two) = two_devices() else {
        eprintln!("no GPU: skipped");
        return;
    };
    let (cpu, err, _) = run(SPLIT, false, &[]);
    assert!(cpu.ends_with("0 out of order\n"), "{cpu}{err}");
    // Split, and split with each part in several passes.
    for words in ["", "1000"] {
        let mut env = vec![
            ("KAVACH_FAKE_GPUS", two.as_str()),
            ("KAVACH_GPU", "on"),
            ("KAVACH_GPU_SPLIT", "on"),
            ("KAVACH_PAR_LOG", "1"),
        ];
        if !words.is_empty() {
            env.push(("KAVACH_GPU_WORDS", words));
        }
        let (out, err, _) = run(SPLIT, true, &env);
        assert_eq!(out, cpu, "{err}");
        assert!(
            err.contains("kavach: a map of 5000 items split across GPUs: 0 (2304), 1 (2696)"),
            "{err}"
        );
        // `check` runs the CPU too, and compares.
        env[1] = ("KAVACH_GPU", "check");
        let (out, err, _) = run(SPLIT, true, &env);
        assert_eq!(out, cpu, "{err}");
    }
    // The machine's own GPUs, as the system ranks them: software ones
    // take no part, so this splits only on a machine with two others.
    let env = [("KAVACH_GPU", "on"), ("KAVACH_GPU_SPLIT", "on")];
    let (out, err, _) = run(SPLIT, true, &env);
    assert_eq!(out, cpu, "{err}");
    // Not split: `off`.
    let env = [
        ("KAVACH_FAKE_GPUS", two.as_str()),
        ("KAVACH_GPU", "on"),
        ("KAVACH_GPU_SPLIT", "off"),
        ("KAVACH_PAR_LOG", "1"),
    ];
    let (out, err, _) = run(SPLIT, true, &env);
    assert_eq!(out, cpu, "{err}");
    assert!(!err.contains("split"), "{err}");
}

const CHOSEN: &str = "use std.gpu

gpu fn twice(n: Int32) -> Int32 {
    n * 2 + 1
}

fn main() uses Gpu {
    let ns: [Int32] = []
    for i in 0..5000 {
        ns.push(Int32(i))
    }
    const a = ns.map(twice, on: gpu.devices()[1])
    const b = on gpu(0) ns.map(twice)
    print(`{a[4999]} {b[4999]}`)
}
";

#[test]
fn a_map_whose_device_is_chosen_never_splits() {
    let Some(two) = two_devices() else {
        eprintln!("no GPU: skipped");
        return;
    };
    let env = [
        ("KAVACH_FAKE_GPUS", two.as_str()),
        ("KAVACH_GPU", "on"),
        ("KAVACH_GPU_SPLIT", "on"),
        ("KAVACH_PAR_LOG", "1"),
    ];
    let (out, err, _) = run(CHOSEN, true, &env);
    assert_eq!(out, "9999 9999\n", "{err}");
    assert!(!err.contains("split"), "{err}");
    // `KAVACH_GPU` naming a device chooses it too.
    let env = [
        ("KAVACH_FAKE_GPUS", two.as_str()),
        ("KAVACH_GPU", "1"),
        ("KAVACH_GPU_SPLIT", "on"),
        ("KAVACH_PAR_LOG", "1"),
    ];
    let (out, err, _) = run(SPLIT, true, &env);
    assert!(out.ends_with("0 out of order\n"), "{out}{err}");
    assert!(!err.contains("split"), "{err}");
}

const OVERFLOW: &str = "gpu fn bump(n: Int32) -> Int32 {
    if n == 100 or n == 3000 {
        return n + 2147483600
    }
    n
}

fn main() {
    let ns: [Int32] = []
    for i in 0..5000 {
        ns.push(Int32(i))
    }
    print(ns.map(bump).len())
}
";

#[test]
fn a_panic_in_a_part_is_the_cpus() {
    let Some(two) = two_devices() else {
        eprintln!("no GPU: skipped");
        return;
    };
    // Items 100 and 3000 overflow, one in each part: the CPU's panic
    // names the first.
    let (cpu, cpu_err, cpu_code) = run(OVERFLOW, false, &[]);
    assert_eq!((cpu.as_str(), cpu_code), ("", 101), "{cpu_err}");
    let env = [
        ("KAVACH_FAKE_GPUS", two.as_str()),
        ("KAVACH_GPU", "on"),
        ("KAVACH_GPU_SPLIT", "on"),
        ("KAVACH_PAR_LOG", "1"),
    ];
    let (out, err, code) = run(OVERFLOW, true, &env);
    assert_eq!((out.as_str(), code), ("", 101), "{err}");
    assert!(err.contains("split across GPUs"), "{err}");
    let panic = |e: &str| {
        e.lines()
            .find(|l| l.starts_with("panic:"))
            .map(String::from)
    };
    assert!(
        panic(&cpu_err).is_some_and(|p| p.contains("100 + 2147483600")),
        "{cpu_err}"
    );
    assert_eq!(panic(&err), panic(&cpu_err), "{err}\n{cpu_err}");
}

#[test]
fn fake_devices_that_cant_open_leave_the_map_on_the_cpu() {
    let fake =
        "AMD Radeon RX 7900:0x1002:discrete:vulkan;AMD Radeon RX 7800:0x1002:discrete:vulkan";
    let env = [
        ("KAVACH_FAKE_GPUS", fake),
        ("KAVACH_GPU", "on"),
        ("KAVACH_GPU_SPLIT", "on"),
        ("KAVACH_PAR_LOG", "1"),
    ];
    let (cpu, _, _) = run(SPLIT, false, &[]);
    let (out, err, _) = run(SPLIT, true, &env);
    assert_eq!(out, cpu, "{err}");
    assert!(err.contains("GPU 0 (AMD Radeon RX 7900) skipped"), "{err}");
    assert!(err.contains("GPU 1 (AMD Radeon RX 7800) skipped"), "{err}");
    assert!(!err.contains("split across"), "{err}");
}
