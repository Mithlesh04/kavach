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
    /// default, lets the runtime choose; a vendor's name or a device's
    /// number chooses the device (spec 15.6). `KAVACH_GPU` at launch wins.
    pub gpu: Option<String>,
}

impl RunConfig {
    /// Whether `gpu = "cpu"` (or `"off"`, in any case).
    pub fn on_cpu(&self) -> bool {
        use kavach_runtime_core::gpu_setting::{self, GpuSetting};
        self.gpu.as_deref().and_then(gpu_setting::read) == Some(GpuSetting::Cpu)
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
        use kavach_runtime_core::gpu_setting::{self, GpuSetting};
        match self.gpu.as_deref() {
            None => None,
            Some(t) if t.trim().eq_ignore_ascii_case("auto") => None,
            Some(t)
                if matches!(
                    gpu_setting::read(t),
                    Some(GpuSetting::Cpu | GpuSetting::Number(_) | GpuSetting::Vendor(_))
                ) =>
            {
                None
            }
            Some(other) => Some(gpu_setting::takes(&format!("`[run] gpu = \"{other}\"`"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RunConfig;

    /// `[run] gpu` is read in any case, as `KAVACH_GPU` is (spec 15.2).
    #[test]
    fn gpu_is_read_in_any_case() {
        let run = |gpu: &str| RunConfig {
            gpu: Some(gpu.into()),
            ..RunConfig::default()
        };
        for cpu in ["cpu", "CPU", "Off"] {
            assert!(run(cpu).on_cpu(), "{cpu}");
            assert_eq!(run(cpu).wrong(), None, "{cpu}");
        }
        for other in ["auto", "AUTO", "Nvidia", "1"] {
            assert!(!run(other).on_cpu(), "{other}");
            assert_eq!(run(other).wrong(), None, "{other}");
        }
        assert!(run("fast").wrong().is_some());
    }
}
