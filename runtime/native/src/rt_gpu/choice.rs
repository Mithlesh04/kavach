// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Choosing a GPU in native code (spec 15.6): the place `on gpu(n) x`,
//! `gpu.on(d) { … }` and `xs.map(f, on: d)` put on the task's scope
//! stack, which `runtime/gpu` asks when it picks a device, and the check
//! at the program's start that every device it asks for is there.

use crate::heap::{release, with_str};
use crate::rt::panic_with;
use crate::rt_scopes::{current_place, push};
use kavach_runtime_core::fs_scope::{GpuPlace, Scope};

/// `GpuPlace`: device `n` for the GPU work until the scope ends; gives
/// the depth to go back to. A device the machine doesn't have panics.
pub(super) fn place(n: i64, line: i32) -> i64 {
    if let Some(why) = kavach_runtime_gpu::missing(n) {
        panic_with(&why, line);
    }
    kavach_runtime_core::set_place_hook(current_place);
    push(Scope::Place {
        cores: 0,
        gpu: GpuPlace::Device(n as usize),
    })
}

/// `GpuStart`: `highest`, the highest GPU number the code writes (or -1),
/// `place` saying where, and `file`, the text of `[run] gpu`. A device
/// that isn't there stops the program, saying what the machine has.
pub(super) fn start(highest: i64, place: i64, file: i64) -> i64 {
    let text = |t: i64| {
        let s = with_str(t, str::to_string);
        release(t);
        s
    };
    let (place, file) = (text(place), text(file));
    let highest = usize::try_from(highest).ok();
    let file = (!file.is_empty()).then_some(file.as_str());
    if let Err(why) = kavach_runtime_gpu::start(highest, &place, file) {
        stop(&why);
    }
    0
}

/// `KAVACH_GPU` naming a device that isn't there stops the program.
pub(super) fn check_launch() {
    if let Err(why) = kavach_runtime_gpu::check_launch() {
        stop(&why);
    }
}

/// Stops the program at its start, saying why (exit code 1).
fn stop(why: &str) -> ! {
    eprintln!("error: {why}");
    std::process::exit(1);
}
