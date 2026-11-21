// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A `map` too big for one pass of the GPU runs in several (spec 15.2),
//! in native code. `KAVACH_GPU_WORDS` makes a pass small, so a short
//! list needs several. Its results must be the CPU's, also when they
//! are kept in a local to be read later (decision 0055) and are records,
//! several words an item. Without a GPU the runtime uses the CPU, so
//! these pass either way.

use std::process::Command;

/// Records kept in locals, read after the `map`s; a 1000-item list in
/// passes of 256 words.
const RECORDS: &str = "type Ball {
    y: Float32
    vy: Float32
}

gpu fn fall(b: Ball) -> Ball {
    const vy = b.vy - 0.25
    Ball(y: b.y + vy, vy: vy)
}

gpu fn pair(x: Int32) -> (Int32, Int32) {
    (x, x * 3)
}

fn main() {
    let balls: [Ball] = []
    let ns: [Int32] = []
    for i in 0..1000 {
        balls.push(Ball(y: Float32(i), vy: 0.5))
        ns.push(Int32(i))
    }
    const fallen = balls.map(fall)
    const pairs = ns.map(pair)
    print(fallen[0])
    print(fallen[999])
    print(pairs[999])
    let t = 0
    for p in pairs {
        t += Int(p.1)
    }
    print(`{fallen.len()} {t}`)
}
";

/// Runs `src` with `run --native` and `env`: its output and errors.
fn run(src: &str, env: &[(&str, &str)]) -> (String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kavach-gpu-passes-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.args(["run", "--native"]).arg(dir.join("main.kav"));
    cmd.env_remove("KAVACH_GPU").env_remove("KAVACH_FAKE_GPUS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (text(&out.stdout), text(&out.stderr))
}

#[test]
fn records_kept_from_a_map_of_several_passes_are_the_cpus() {
    let (want, err) = run(RECORDS, &[("KAVACH_GPU", "cpu")]);
    assert_eq!(
        want, "Ball(y: 0.25, vy: 0.25)\nBall(y: 999.25, vy: 0.25)\n(999, 2997)\n1000 1498500\n",
        "{err}"
    );
    for words in ["256", "999"] {
        let (got, err) = run(
            RECORDS,
            &[("KAVACH_GPU", "on"), ("KAVACH_GPU_WORDS", words)],
        );
        assert_eq!(got, want, "KAVACH_GPU_WORDS={words}: {err}");
    }
}
