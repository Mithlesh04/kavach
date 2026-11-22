// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Splitting one big `map` across the GPUs (spec 15.6, decision 0144),
//! as plain functions: whether its size pays, each device's speed, and
//! the run of items each device takes. `devices.rs` gives them the
//! devices, and `map.rs` runs the parts.

use crate::WORKGROUP;
use crate::listing::Kind;
use std::ops::Range;

/// The least work worth splitting, in nanoseconds of the CPU: a second.
const PAYS_NS: u64 = 1_000_000_000;

/// The least work for each 32-bit word of an item and its result that
/// crosses to a GPU and back.
const WORK_PER_WORD: u64 = 100;

/// What a part costs beyond its work, in nanoseconds: its thread, its
/// dispatch and reading its results back.
const PART_NS: f64 = 2_000_000.0;

/// The fewest items a device's part may have: one workgroup.
pub(crate) const LEAST_PART: usize = WORKGROUP as usize;

/// `KAVACH_GPU_SPLIT`: whether a `map` that may split does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Mode {
    /// When its size pays.
    Auto,
    /// At any size, its devices opened first: for tests.
    Always,
    Never,
}

impl Mode {
    /// The mode `KAVACH_GPU_SPLIT`'s text gives, in any case; `Auto` when
    /// it is missing or reads as neither.
    pub(crate) fn of(text: Option<&str>) -> Mode {
        match text.map(|t| t.trim().to_ascii_lowercase()).as_deref() {
            Some("on") => Mode::Always,
            Some("off") => Mode::Never,
            _ => Mode::Auto,
        }
    }
}

/// A kernel's work for each item, from its `// work <n>` header line: 1
/// when it has none.
pub fn work_per_item(kernel: &str) -> u64 {
    (kernel.lines().take_while(|l| l.starts_with("//")))
        .find_map(|l| l.strip_prefix("// work "))
        .and_then(|w| w.trim().parse::<u64>().ok())
        .unwrap_or(1)
}

/// About how long `n` items of `work` each take the CPU, in nanoseconds:
/// a unit of work is about 0.1 ns (`runtime/native/src/rt_gpu.rs`).
pub fn cpu_ns(n: usize, work: u64) -> u64 {
    (n as u64).saturating_mul(work) / 10
}

/// Whether a `map` of `n` items pays for splitting, each of `work` units
/// and `words` 32-bit words of item and result together.
pub(crate) fn pays(mode: Mode, n: usize, work: u64, words: usize) -> bool {
    match mode {
        Mode::Never => false,
        Mode::Always => n >= 2 * LEAST_PART,
        Mode::Auto => {
            cpu_ns(n, work) >= PAYS_NS && work >= WORK_PER_WORD.saturating_mul(words.max(1) as u64)
        }
    }
}

/// A device's speed guessed from its kind, before it has done a part: a
/// discrete GPU as 8 integrated ones.
pub(crate) fn guess(kind: Kind) -> f64 {
    match kind {
        Kind::Discrete => 8.0,
        _ => 1.0,
    }
}

/// The speeds to size the parts by: the measured ones when every device
/// has one (above 0), or else every device's guess, as the two aren't
/// in the same units. `each` is `(measured, guess)`.
pub(crate) fn speeds(each: &[(f64, f64)]) -> Vec<f64> {
    let measured = each.iter().all(|(m, _)| *m > 0.0);
    (each.iter())
        .map(|&(m, g)| if measured { m } else { g })
        .collect()
}

/// A new measure `now` of a device's speed, averaged with the last one
/// (`0` when there is none).
pub(crate) fn averaged(last: f64, now: f64) -> f64 {
    match last > 0.0 {
        true => (last + now) / 2.0,
        false => now,
    }
}

/// Whether `parts` of `n` items of `work` each, at measured `speeds`
/// (work per nanosecond), end a tenth sooner than the fastest device
/// alone would, counting each part's own cost.
pub(crate) fn beats_one(
    n: usize,
    work: u64,
    speeds: &[f64],
    parts: &[Option<Range<usize>>],
) -> bool {
    let work = work as f64;
    let fastest = speeds.iter().copied().fold(0.0, f64::max);
    if fastest <= 0.0 {
        return false;
    }
    let alone = n as f64 * work / fastest;
    let split = (parts.iter().zip(speeds))
        .filter_map(|(r, s)| r.as_ref().map(|r| r.len() as f64 * work / s))
        .fold(0.0, f64::max);
    (split + PART_NS) * 1.1 < alone
}

/// The items each device takes of `n`, by `speeds` (in the devices'
/// order, best first): runs of whole items, in order, each as long as
/// the device's share, starting at a multiple of a workgroup. A device
/// whose share would be under `LEAST_PART` is left out (`None`), the
/// slowest first. Fewer than two parts left means no split: `None`.
pub(crate) fn parts(n: usize, speeds: &[f64]) -> Option<Vec<Option<Range<usize>>>> {
    let mut kept: Vec<bool> = speeds.iter().map(|s| *s > 0.0).collect();
    loop {
        let total: f64 = (speeds.iter().zip(&kept))
            .filter(|(_, k)| **k)
            .map(|(s, _)| s)
            .sum();
        let share = |i: usize| n as f64 * speeds[i] / total;
        let small = (0..speeds.len())
            .filter(|&i| kept[i] && share(i) < LEAST_PART as f64)
            .min_by(|&a, &b| speeds[a].total_cmp(&speeds[b]));
        match small {
            Some(i) => kept[i] = false,
            None => break,
        }
    }
    let taking: Vec<usize> = (0..speeds.len()).filter(|&i| kept[i]).collect();
    if taking.len() < 2 {
        return None;
    }
    let total: f64 = taking.iter().map(|&i| speeds[i]).sum();
    let mut out = vec![None; speeds.len()];
    let (mut start, mut sofar) = (0, 0.0);
    for (k, &i) in taking.iter().enumerate() {
        sofar += speeds[i];
        let end = match k + 1 == taking.len() {
            true => n,
            false => {
                let at = (n as f64 * sofar / total) as usize;
                (at / LEAST_PART * LEAST_PART).clamp(start, n)
            }
        };
        out[i] = Some(start..end);
        start = end;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_split_pays_for_heavy_work_on_many_items() {
        // The mode, the items, the work for each, the words moved for
        // each, and whether it splits.
        let table = [
            (Mode::Auto, 4_000_000, 3_000, 2, true),
            // Too little in all: under a second of the CPU's time.
            (Mode::Auto, 4_000_000, 2_000, 2, false),
            (Mode::Auto, 1_000, 30_000_000, 2, true),
            // Too light for the words it moves.
            (Mode::Auto, 100_000_000, 150, 2, false),
            (Mode::Auto, 100_000_000, 200, 2, true),
            (Mode::Auto, 100_000_000, 799, 8, false),
            // `on` splits at any size with two workgroups; `off` never.
            (Mode::Always, 512, 1, 2, true),
            (Mode::Always, 511, 1, 2, false),
            (Mode::Never, 100_000_000, 100_000, 2, false),
        ];
        for (mode, n, work, words, want) in table {
            assert_eq!(
                pays(mode, n, work, words),
                want,
                "{mode:?} {n} {work} {words}"
            );
        }
    }

    #[test]
    fn the_mode_and_the_work_are_read_from_their_text() {
        let table = [
            (Some("on"), Mode::Always),
            (Some(" ON "), Mode::Always),
            (Some("Off"), Mode::Never),
            (Some("maybe"), Mode::Auto),
            (None, Mode::Auto),
        ];
        for (text, want) in table {
            assert_eq!(Mode::of(text), want, "{text:?}");
        }
        assert_eq!(work_per_item("// work 1200\n// out 4\nfn main() {}"), 1200);
        assert_eq!(work_per_item("// out 4\n// work 7\n"), 7);
        assert_eq!(work_per_item("fn main() {}\n// work 7\n"), 1);
        assert_eq!(cpu_ns(4_000_000, 3_000), 1_200_000_000);
    }

    #[test]
    fn speeds_are_measured_ones_only_when_every_device_has_one() {
        assert_eq!(speeds(&[(0.0, 8.0), (0.0, 1.0)]), [8.0, 1.0]);
        assert_eq!(speeds(&[(3.5, 8.0), (0.0, 1.0)]), [8.0, 1.0]);
        assert_eq!(speeds(&[(3.5, 8.0), (0.5, 1.0)]), [3.5, 0.5]);
        assert_eq!(guess(Kind::Discrete), 8.0);
        assert_eq!(guess(Kind::Integrated), 1.0);
        assert_eq!(averaged(0.0, 4.0), 4.0);
        assert_eq!(averaged(2.0, 4.0), 3.0);
    }

    #[test]
    fn measured_parts_must_beat_the_fastest_device_alone() {
        // 4 million items of 1,000 units, on devices doing 4 and
        // 1 unit a nanosecond: 1 s alone, 0.8 s in parts.
        let (n, work) = (4_000_000, 1_000);
        let parts = [Some(0..3_200_000), Some(3_200_000..4_000_000)];
        assert!(beats_one(n, work, &[4.0, 1.0], &parts));
        // A slow second device: its part ends last.
        assert!(!beats_one(n, work, &[4.0, 0.1], &parts));
        // A short job: the parts' own cost outweighs what they save.
        let parts = [Some(0..32_000), Some(32_000..40_000)];
        assert!(!beats_one(40_000, work, &[4.0, 1.0], &parts));
        assert!(!beats_one(n, work, &[0.0, 0.0], &parts));
    }

    #[test]
    fn the_parts_follow_the_speeds_in_order() {
        type Parts = Option<Vec<Option<Range<usize>>>>;
        // The items, the speeds, and the parts.
        let table: Vec<(usize, Vec<f64>, Parts)> = vec![
            (
                4096,
                vec![1.0, 1.0],
                Some(vec![Some(0..2048), Some(2048..4096)]),
            ),
            // Parts start at a multiple of a workgroup; the last takes
            // what is left.
            (
                1000,
                vec![1.0, 1.0],
                Some(vec![Some(0..256), Some(256..1000)]),
            ),
            (
                1300,
                vec![1.0, 1.0],
                Some(vec![Some(0..512), Some(512..1300)]),
            ),
            (
                9_000_000,
                vec![8.0, 1.0],
                Some(vec![Some(0..8_000_000), Some(8_000_000..9_000_000)]),
            ),
            (
                3 * 4096,
                vec![1.0, 1.0, 1.0],
                Some(vec![Some(0..4096), Some(4096..8192), Some(8192..12288)]),
            ),
            // A share under a workgroup leaves its device out, the
            // slowest first.
            (
                2048,
                vec![8.0, 1.0, 8.0],
                Some(vec![Some(0..1024), None, Some(1024..2048)]),
            ),
            (2048, vec![8.0, 1.0], None),
            (511, vec![1.0, 1.0], None),
            (100_000, vec![1.0], None),
            (100_000, vec![], None),
        ];
        for (n, speeds, want) in table {
            let got = parts(n, &speeds);
            assert_eq!(got, want, "{n} {speeds:?}");
            // Every item in exactly one part, in order.
            if let Some(got) = got {
                let runs: Vec<Range<usize>> = got.into_iter().flatten().collect();
                assert_eq!(runs.first().map(|r| r.start), Some(0));
                assert_eq!(runs.last().map(|r| r.end), Some(n));
                assert!(runs.windows(2).all(|w| w[0].end == w[1].start));
                assert!(runs.iter().all(|r| r.len() >= LEAST_PART));
            }
        }
    }
}
