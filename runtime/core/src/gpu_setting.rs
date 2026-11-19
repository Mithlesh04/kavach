// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What `KAVACH_GPU` and `[run] gpu` ask for (spec 15.2, 15.6): the
//! system's choice, the CPU, a device's number, or the best device of a
//! vendor. Read here once, for the compiler's check of `kavach.toml` and
//! for the runtime alike.

/// What a GPU setting asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuSetting {
    /// The system chooses (`auto`; `on` and `check` too, which say only
    /// whether to use a GPU).
    System,
    /// Every GPU `map` on the CPU.
    Cpu,
    /// The device of this number, as `system.gpus()` lists them.
    Number(usize),
    /// The best device of this vendor, named as `system.gpus()` names it.
    Vendor(&'static str),
}

/// The vendors a setting may name, as `system.gpus()` writes them.
pub const VENDORS: [&str; 7] = [
    "NVIDIA",
    "AMD",
    "Intel",
    "Apple",
    "Qualcomm",
    "ARM",
    "Microsoft",
];

/// What `text` asks for, in any case; `None` when it reads as nothing.
pub fn read(text: &str) -> Option<GpuSetting> {
    let text = text.trim();
    match text.to_ascii_lowercase().as_str() {
        "" | "auto" | "on" | "check" => Some(GpuSetting::System),
        "cpu" | "off" => Some(GpuSetting::Cpu),
        _ => match text.parse::<usize>() {
            Ok(n) => Some(GpuSetting::Number(n)),
            Err(_) => (VENDORS.iter())
                .find(|v| v.eq_ignore_ascii_case(text))
                .map(|v| GpuSetting::Vendor(v)),
        },
    }
}

/// What a setting that can't be read is told (`said`, the setting as it
/// was written): the values it takes.
pub fn takes(said: &str) -> String {
    format!(
        "{said}: write cpu, auto, a device's number from 0, or a vendor: {} (spec 15.6)",
        VENDORS.join(", ").to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_setting_reads_as_what_it_asks_for() {
        let table = [
            ("", Some(GpuSetting::System)),
            ("auto", Some(GpuSetting::System)),
            ("on", Some(GpuSetting::System)),
            ("CHECK", Some(GpuSetting::System)),
            ("cpu", Some(GpuSetting::Cpu)),
            ("Off", Some(GpuSetting::Cpu)),
            ("0", Some(GpuSetting::Number(0))),
            (" 12 ", Some(GpuSetting::Number(12))),
            ("nvidia", Some(GpuSetting::Vendor("NVIDIA"))),
            ("AMD", Some(GpuSetting::Vendor("AMD"))),
            ("intel", Some(GpuSetting::Vendor("Intel"))),
            ("apple", Some(GpuSetting::Vendor("Apple"))),
            ("qualcomm", Some(GpuSetting::Vendor("Qualcomm"))),
            ("Arm", Some(GpuSetting::Vendor("ARM"))),
            ("microsoft", Some(GpuSetting::Vendor("Microsoft"))),
            ("-1", None),
            ("radeon", None),
            ("gpu1", None),
        ];
        for (text, want) in table {
            assert_eq!(read(text), want, "{text:?}");
        }
    }
}
