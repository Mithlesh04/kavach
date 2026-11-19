// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The machine's facts on Linux (spec 14.25), read from `/proc`: no
//! system calls, so no `unsafe`.

use super::Cpu;
use std::collections::HashSet;

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// The value of the first `name: value` line of `text` named `name`.
fn field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == name).then_some(value.trim())
    })
}

pub fn os_version() -> String {
    read("/proc/sys/kernel/osrelease").trim().to_string()
}

pub fn cpu() -> Cpu {
    let info = read("/proc/cpuinfo");
    // ARM systems name the processor elsewhere, or not at all.
    let model = field(&info, "model name").or_else(|| field(&info, "Hardware"));
    let mhz = field(&info, "cpu MHz").and_then(|m| m.parse::<f64>().ok());
    Cpu {
        model: model.unwrap_or("").to_string(),
        vendor: field(&info, "vendor_id").unwrap_or("").to_string(),
        mhz: mhz.map_or(0, |m| m.round() as i64),
    }
}

pub fn hostname() -> String {
    read("/proc/sys/kernel/hostname").trim().to_string()
}

/// The distinct cores: each processor's package and core number.
pub fn physical_cores() -> i64 {
    let info = read("/proc/cpuinfo");
    let mut cores = HashSet::new();
    let mut package = "";
    for line in info.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "physical id" => package = value.trim(),
            "core id" => {
                cores.insert((package.to_string(), value.trim().to_string()));
            }
            _ => {}
        }
    }
    cores.len() as i64
}

/// The memory and the memory available, in bytes.
pub fn memory() -> (i64, i64) {
    let info = read("/proc/meminfo");
    let kb = |name| {
        field(&info, name)
            .and_then(|v| v.trim_end_matches("kB").trim().parse::<i64>().ok())
            .map_or(0, |n| n * 1024)
    };
    (kb("MemTotal"), kb("MemAvailable"))
}

pub fn uptime_ms() -> i64 {
    let text = read("/proc/uptime");
    let seconds = text
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok());
    seconds.map_or(0, |s| (s * 1000.0) as i64)
}
