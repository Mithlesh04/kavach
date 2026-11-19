// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `[run]` in `kavach.toml`: how the program runs (spec 4.9, 15.2).

use serde::Deserialize;

/// `[run]`: the most cores the program uses, and where its GPU maps run.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunConfig {
    /// The most cores the program uses; `None`: all the machine gives it.
    /// `KAVACH_CORES` at launch wins.
    pub cores: Option<u32>,
    /// The most `on threads` rounds running at once in the whole program
    /// (spec 4.9); `None`: 256.
    pub threads: Option<u32>,
    /// `"cpu"` keeps every `map` with a `gpu fn` on the CPU; `"auto"`, the
    /// default, lets the runtime choose. `KAVACH_GPU` at launch wins.
    pub gpu: Option<String>,
}

impl RunConfig {
    /// Whether `gpu = "cpu"`.
    pub fn on_cpu(&self) -> bool {
        self.gpu.as_deref() == Some("cpu")
    }

    /// What is wrong with the section, if anything.
    pub fn wrong(&self) -> Option<String> {
        if self.threads == Some(0) {
            return Some(
                "`[run] threads = 0`: an `on threads` loop runs at least one round: give 1 or more (spec 4.9)"
                    .into(),
            );
        }
        if self.cores == Some(0) {
            return Some(
                "`[run] cores = 0`: a program runs on at least one core: give 1 or more (spec 4.9)"
                    .into(),
            );
        }
        match self.gpu.as_deref() {
            None | Some("cpu" | "auto") => None,
            Some(other) => Some(format!(
                "`[run] gpu = \"{other}\"`: write \"cpu\" to keep every GPU `map` on the CPU, or \"auto\" (spec 15.2)"
            )),
        }
    }
}
