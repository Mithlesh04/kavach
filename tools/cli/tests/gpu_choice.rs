// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Choosing a GPU (spec 15.6), in the interpreter and native code: with
//! AMD, NVIDIA and Intel GPUs that `KAVACH_FAKE_GPUS` lists on any
//! machine, and with Windows' software GPU (WARP) as a real second
//! device. A map gives the CPU's values wherever it runs; what shows
//! where it ran is `KAVACH_PAR_LOG`'s note of each device opened.

use std::process::Command;

const AMD_INTEL: &str =
    "AMD Radeon RX 7900:0x1002:discrete:vulkan;Intel UHD 770:0x8086:integrated:dx12";

const LIST: &str = "use std.gpu

fn main() {
    for d in gpu.devices() {
        print(`{d.index} {d.name} {d.vendor} {d.backend}`)
    }
}
";

/// Runs `src` (as `main.kav`, with `toml` as its `kavach.toml` when
/// given), natively or not, with `env`: its output and errors.
fn run(src: &str, toml: Option<&str>, native: bool, env: &[(&str, &str)]) -> (String, String, i32) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kavach-gpu-choice-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), src).unwrap();
    if let Some(toml) = toml {
        std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    cmd.arg(dir.join("main.kav")).current_dir(&dir);
    cmd.env_remove("KAVACH_GPU").env_remove("KAVACH_FAKE_GPUS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    let code = out.status.code().unwrap_or(-1);
    (text(&out.stdout), text(&out.stderr), code)
}

#[test]
fn gpu_devices_numbers_them_as_system_gpus_lists_them() {
    let fake = format!("{AMD_INTEL};NVIDIA RTX 4090:0x10de:discrete:vulkan");
    let want = "0 AMD Radeon RX 7900 AMD vulkan\n1 NVIDIA RTX 4090 NVIDIA vulkan\n2 Intel UHD 770 Intel dx12\n";
    for native in [false, true] {
        let (out, err, _) = run(LIST, None, native, &[("KAVACH_FAKE_GPUS", &fake)]);
        assert_eq!(out, want, "native: {native}: {err}");
    }
}

const SHADE: &str = "gpu(2) fn shade(x: Float32) -> Float32 {
    x * 2.0
}

fn main() {
    print(\"ran\")
    const xs: [Float32] = [1.0, 2.0]
    print(xs.map(shade))
}
";

#[test]
fn a_missing_gpu_stops_the_program_at_its_start() {
    let has = "this machine has 2: 0 AMD Radeon RX 7900 (AMD, discrete), 1 Intel UHD 770 (Intel, integrated)";
    // The code, the launch and the project's file, each naming a GPU the
    // two fake ones aren't.
    // The program, its `kavach.toml`, its settings, and what it says.
    type Case<'a> = (&'a str, Option<&'a str>, &'a [(&'a str, &'a str)], String);
    let table: [Case; 4] = [
        (SHADE, None, &[], format!("error: this program asks for GPU 2 (`gpu(2) fn shade`), and {has}\n")),
        (
            &SHADE.replace("gpu(2) fn", "gpu fn"),
            None,
            &[("KAVACH_GPU", "nvidia")],
            format!("error: KAVACH_GPU=nvidia asks for a GPU made by NVIDIA, and {has}\n"),
        ),
        (
            &SHADE.replace("gpu(2) fn", "gpu fn"),
            Some("[run]\ngpu = \"5\"\n"),
            &[],
            format!("error: [run] gpu = \"5\" asks for GPU 5, and {has}\n"),
        ),
        (
            &SHADE.replace("gpu(2) fn", "gpu fn"),
            None,
            &[("KAVACH_GPU", "fastest")],
            "error: KAVACH_GPU=fastest: write cpu, auto, a device's number from 0, or a vendor: nvidia, amd, intel, apple, qualcomm, arm, microsoft (spec 15.6)\n".into(),
        ),
    ];
    for (src, toml, env, want) in table {
        let mut env = env.to_vec();
        env.push(("KAVACH_FAKE_GPUS", AMD_INTEL));
        for native in [false, true] {
            let (out, err, code) = run(src, toml, native, &env);
            assert_eq!((out.as_str(), code), ("", 1), "native: {native}: {err}");
            assert!(err.ends_with(&want), "native: {native}: {err}");
        }
    }
}

const CHOOSE: &str = "use std.gpu

gpu fn step(x: Float32) -> Float32 {
    x * 2.0 + 1.0
}

gpu(1) fn shade(x: Float32) -> Float32 {
    x * 3.0
}

fn main() {
    let xs: [Float32] = []
    for i in 0..4096 {
        xs.push(Float32(i % 100))
    }
    const devices = gpu.devices()
    const a = gpu.on(devices[1]) {
        xs.map(step)
    }
    const b = xs.map(step, on: devices[0])
    const c = on gpu(1) xs.map(step)
    const d = xs.map(shade)
    print(`{a[4095]} {b[4095]} {c[4095]} {d[4095]} {a.len()}`)
}
";

/// Windows' software GPU, which every Windows machine with DirectX 12
/// has: a real second device to choose.
const WARP: &str = "Microsoft Basic Render Driver";

#[test]
fn chosen_gpus_give_the_cpus_values_and_one_missing_is_skipped() {
    // A fake AMD GPU first: it can't be opened, so it is skipped.
    let fake = format!("AMD Radeon RX 7900:0x1002:discrete:vulkan;{WARP}:0x1414:software:dx12");
    let env = [
        ("KAVACH_FAKE_GPUS", fake.as_str()),
        ("KAVACH_GPU", "on"),
        ("KAVACH_PAR_LOG", "1"),
    ];
    let (cpu, err, _) = run(CHOOSE, None, false, &env);
    assert!(
        cpu.starts_with("191") && cpu.ends_with(" 4096\n"),
        "{cpu}{err}"
    );
    let (out, err, _) = run(CHOOSE, None, true, &env);
    assert_eq!(out, cpu, "{err}");
    assert!(err.contains("GPU 0 (AMD Radeon RX 7900) skipped"), "{err}");
    // On Windows, the chosen work ran on WARP, opened as device 1.
    if cfg!(windows) && real_gpus().iter().any(|g| g.starts_with(WARP)) {
        assert!(
            err.contains(&format!("GPU 1 ({WARP}, dx12) opened")),
            "{err}"
        );
    }
}

/// The machine's own GPUs, as `system.gpus()` lists them.
fn real_gpus() -> Vec<String> {
    let src = "use std.system\n\nfn main() {\n    for g in system.gpus() {\n        print(g.name)\n    }\n}\n";
    let (out, _, _) = run(src, None, false, &[]);
    out.lines().map(String::from).collect()
}

/// Maps with no choice in the code: `[run] gpu` chooses.
const PLAIN: &str = "gpu fn step(x: Float32) -> Float32 {
    x * 2.0 + 1.0
}

fn main() {
    let xs: [Float32] = []
    for i in 0..4096 {
        xs.push(Float32(i % 100))
    }
    const ys = xs.map(step)
    print(`{ys[4095]} {ys.map(step).sum(x => x)}`)
}
";

#[test]
fn each_real_gpu_chosen_by_number_gives_the_cpus_values() {
    let n = real_gpus().len();
    let (cpu, err, _) = run(PLAIN, None, false, &[]);
    assert!(cpu.starts_with("191"), "{cpu}{err}");
    for i in 0..n {
        let toml = format!("[run]\ngpu = \"{i}\"\n");
        let env = [("KAVACH_GPU", "on"), ("KAVACH_PAR_LOG", "1")];
        let (out, err, _) = run(PLAIN, Some(&toml), true, &env);
        assert_eq!(out, cpu, "GPU {i}: {err}");
    }
}
