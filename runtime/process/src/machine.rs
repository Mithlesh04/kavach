// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What machine the program runs on (spec 14.25, decision 0110): the
//! numbers and text `std.system` is built on, by a fixed number for each
//! fact. Nothing fails: what a system doesn't say is `""` or 0. Each
//! system's own calls are in `machine_<system>.rs`.

#[cfg(target_os = "linux")]
#[path = "machine_linux.rs"]
mod facts;
#[cfg(target_os = "macos")]
#[path = "machine_macos.rs"]
mod facts;
#[cfg(windows)]
#[path = "machine_windows.rs"]
mod facts;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod facts {
    pub fn os_version() -> String {
        String::new()
    }
    pub fn cpu() -> super::Cpu {
        super::Cpu::default()
    }
    pub fn hostname() -> String {
        String::new()
    }
    pub fn physical_cores() -> i64 {
        0
    }
    pub fn memory() -> (i64, i64) {
        (0, 0)
    }
    pub fn uptime_ms() -> i64 {
        0
    }
}

/// The processor: its name, its maker and its rated speed in MHz.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cpu {
    pub model: String,
    pub vendor: String,
    pub mhz: i64,
}

/// Fact `which` as text: 0 the system's name, 1 its version, 2 the
/// architecture, 3 the processor's model, 4 its maker, 5 the hostname.
pub fn text(which: i64) -> String {
    match which {
        0 => std::env::consts::OS.to_string(),
        1 => facts::os_version(),
        2 => std::env::consts::ARCH.to_string(),
        3 => facts::cpu().model,
        4 => facts::cpu().vendor,
        5 => facts::hostname(),
        _ => String::new(),
    }
}

/// Fact `which` as a number: 0 the threads that can run at once, 1 the
/// cores behind them, 2 the memory in bytes, 3 the memory available, 4
/// the processor's speed in MHz, 5 the milliseconds since the machine
/// started.
pub fn number(which: i64) -> i64 {
    match which {
        0 => logical(),
        // Never more than the logical count: under a container's CPU
        // limit both count what this program may use (spec 14.25).
        1 => match facts::physical_cores() {
            n if n > 0 => n.min(logical()),
            _ => logical(),
        },
        2 => facts::memory().0,
        3 => facts::memory().1,
        4 => facts::cpu().mhz,
        5 => facts::uptime_ms(),
        _ => 0,
    }
}

/// How many threads can run at once for this program.
fn logical() -> i64 {
    kavach_runtime_core::cores() as i64
}

/// The processor time this program has spent, on all its threads, in
/// milliseconds.
pub fn cpu_ms() -> i64 {
    crate::system::cpu_self()
}
