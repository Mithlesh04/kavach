// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `system.gpus()` with `KAVACH_FAKE_GPUS` (spec 14.25): GPUs of every
//! vendor on any machine, discrete first, then integrated, the rest last,
//! ties in the order listed, in the interpreter and native code. Nothing
//! is opened: the list replaces the machine's.

use std::process::Command;

const PROGRAM: &str = "use std.system

fn kind(k: GpuKind) -> String {
    match k {
        GpuKind.Discrete => \"discrete\",
        GpuKind.Integrated => \"integrated\",
        GpuKind.Virtual => \"virtual\",
        GpuKind.Other => \"other\",
        GpuKind.Software => \"software\",
    }
}

fn main() uses Console, System {
    for g in system.gpus() {
        print(`{g.name}|{g.vendor}|{kind(g.kind)}|{g.backend}`)
    }
}
";

/// Listed worst first, with one entry that doesn't read.
const FAKE: &str = "Basic Render:0x1414:software:dx12;\
                    Intel UHD 770:0x8086:integrated:dx12;\
                    AMD Radeon RX 7900:0x1002:discrete:vulkan;\
                    broken entry;\
                    Cloud GPU:0x1af4:virtual:vulkan;\
                    NVIDIA RTX 4090:0x10de:Discrete:vulkan";

const WANT: &str = "AMD Radeon RX 7900|AMD|discrete|vulkan
NVIDIA RTX 4090|NVIDIA|discrete|vulkan
Intel UHD 770|Intel|integrated|dx12
Cloud GPU||virtual|vulkan
Basic Render|Microsoft|software|dx12
";

#[test]
fn a_fake_gpu_list_is_ranked_like_the_machines() {
    let dir = std::env::temp_dir().join(format!("kavach-fake-gpus-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("gpus.kav");
    std::fs::write(&file, PROGRAM).unwrap();
    for native in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd
            .arg(&file)
            .current_dir(&dir)
            .env("KAVACH_FAKE_GPUS", FAKE)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        assert_eq!(text, WANT, "native: {native}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
