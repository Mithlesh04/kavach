// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Which device gets GPU work (spec 15.6), as plain functions over what
//! is seen of the devices, so the ranking and the choosing are tested
//! with any GPUs on any machine. `devices.rs` gives them the table.

use kavach_runtime_core::gpu_setting::{self, GpuSetting};

/// What is known of one device now, in the table's order (best first).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Seen {
    /// Its maker, as `system.gpus()` names it.
    pub(crate) vendor: &'static str,
    /// It draws on the processor: used only when chosen.
    pub(crate) software: bool,
    /// It can't be opened, or was lost.
    pub(crate) failed: bool,
    pub(crate) open: bool,
    /// Its work sent and not finished.
    pub(crate) busy: usize,
}

/// The devices automatic work may go to, best first.
fn usable(seen: &[Seen]) -> Vec<usize> {
    (0..seen.len())
        .filter(|&i| !seen[i].software && !seen[i].failed)
        .collect()
}

/// Device `n`, then the others automatic work may go to, best first:
/// where a chosen device that fails sends its work.
pub(crate) fn from(n: usize, seen: &[Seen]) -> Vec<usize> {
    let rest = usable(seen).into_iter().filter(|&i| i != n);
    std::iter::once(n).chain(rest).collect()
}

/// The best device automatic work may go to.
pub(crate) fn from_best(seen: &[Seen]) -> Option<usize> {
    usable(seen).first().copied()
}

/// Where automatic work goes: the devices to try in order, the least
/// busy open one first (the best on a tie), and the device to open in
/// the background when every open one is busy, if any.
pub(crate) fn least_busy(seen: &[Seen]) -> (Vec<usize>, Option<usize>) {
    let order = usable(seen);
    let Some(&best) = order.first() else {
        return (order, None);
    };
    if !seen[best].open || seen[best].busy == 0 {
        return (order, None);
    }
    let open = order.iter().copied().filter(|&i| seen[i].open);
    let idlest = open.min_by_key(|&i| seen[i].busy).unwrap_or(best);
    let later = match seen[idlest].busy {
        0 => None,
        _ => order.iter().copied().find(|&i| !seen[i].open),
    };
    let rest = order.iter().copied().filter(|&i| i != idlest);
    (std::iter::once(idlest).chain(rest).collect(), later)
}

/// The device a kernel of a `gpu(n) fn` names on its `// gpu n` line.
pub(crate) fn kernel_device(kernel: &str) -> Option<usize> {
    (kernel.lines().take_while(|l| l.starts_with("//")))
        .find_map(|l| l.strip_prefix("// gpu "))
        .and_then(|n| n.trim().parse().ok())
}

/// The setting that names a device, with its name for messages:
/// `KAVACH_GPU` when it names one, or else `[run] gpu`.
pub(crate) fn setting_text(
    env: Option<&str>,
    file: Option<&str>,
) -> Option<(&'static str, String)> {
    let names = |t: &str| {
        matches!(
            gpu_setting::read(t),
            Some(GpuSetting::Number(_) | GpuSetting::Vendor(_))
        )
    };
    match (env, file) {
        (Some(e), _) if names(e) => Some(("KAVACH_GPU", e.to_string())),
        (_, Some(f)) if names(f) => Some(("[run] gpu", f.to_string())),
        _ => None,
    }
}

/// The device `text` names among `seen`: `None` when it names none
/// (`cpu`, `auto`), `Err` when it can't be read or isn't there.
pub(crate) fn number_of(text: &str, seen: &[Seen]) -> Result<Option<usize>, Missing> {
    match gpu_setting::read(text) {
        None => Err(Missing::Unread),
        Some(GpuSetting::Number(n)) if n < seen.len() => Ok(Some(n)),
        Some(GpuSetting::Number(n)) => Err(Missing::Number(n)),
        Some(GpuSetting::Vendor(v)) => (seen.iter().position(|s| s.vendor == v))
            .map(Some)
            .ok_or(Missing::Vendor(v)),
        Some(_) => Ok(None),
    }
}

/// Why a setting names no device.
#[derive(Debug, PartialEq)]
pub(crate) enum Missing {
    Unread,
    Number(usize),
    Vendor(&'static str),
}

/// What the machine has, for a message: `names` are the devices'.
fn machine_has(names: &[String]) -> String {
    match names.len() {
        0 => "this machine has none".into(),
        n => {
            let each: Vec<String> = (names.iter().enumerate())
                .map(|(i, name)| format!("{i} {name}"))
                .collect();
            format!("this machine has {n}: {}", each.join(", "))
        }
    }
}

/// The start's check (spec 15.6): the highest GPU number written in the
/// code (`place` says where), and `KAVACH_GPU` and `[run] gpu`. `Err`
/// says what is missing, and what the machine has.
pub(crate) fn check_start(
    highest: Option<usize>,
    place: &str,
    env: Option<&str>,
    file: Option<&str>,
    seen: &[Seen],
    names: &[String],
) -> Result<(), String> {
    let has = machine_has(names);
    if let Some(n) = highest.filter(|&n| n >= seen.len()) {
        return Err(format!(
            "this program asks for GPU {n} ({place}), and {has}"
        ));
    }
    let env_names = env.is_some_and(|e| setting_text(Some(e), None).is_some());
    let settings = [
        ("KAVACH_GPU", env),
        ("[run] gpu", file.filter(|_| !env_names)),
    ];
    for (name, text) in settings {
        let Some(text) = text else { continue };
        let said = match name {
            "KAVACH_GPU" => format!("KAVACH_GPU={text}"),
            _ => format!("[run] gpu = \"{text}\""),
        };
        match number_of(text, seen) {
            Ok(_) => {}
            Err(Missing::Unread) => return Err(gpu_setting::takes(&said)),
            Err(Missing::Number(n)) => return Err(format!("{said} asks for GPU {n}, and {has}")),
            Err(Missing::Vendor(v)) => {
                return Err(format!("{said} asks for a GPU made by {v}, and {has}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device: its vendor, and whether it is software, failed, open,
    /// and how busy.
    fn dev(vendor: &'static str, flags: &str, busy: usize) -> Seen {
        Seen {
            vendor,
            software: flags.contains('s'),
            failed: flags.contains('f'),
            open: flags.contains('o'),
            busy,
        }
    }

    #[test]
    fn automatic_work_goes_to_the_least_busy_device() {
        // The devices; the order to try; the one to open meanwhile.
        let table: Vec<(Vec<Seen>, Vec<usize>, Option<usize>)> = vec![
            // Nothing open yet: the best, which opens now.
            (
                vec![dev("NVIDIA", "", 0), dev("Intel", "", 0)],
                vec![0, 1],
                None,
            ),
            // The best is idle.
            (
                vec![dev("NVIDIA", "o", 0), dev("Intel", "o", 0)],
                vec![0, 1],
                None,
            ),
            // The best is busy and the next is idle.
            (
                vec![dev("NVIDIA", "o", 2), dev("Intel", "o", 0)],
                vec![1, 0],
                None,
            ),
            // Both busy: the least; a tie goes to the best.
            (
                vec![dev("NVIDIA", "o", 3), dev("Intel", "o", 1)],
                vec![1, 0],
                None,
            ),
            (
                vec![dev("NVIDIA", "o", 1), dev("Intel", "o", 1)],
                vec![0, 1],
                None,
            ),
            // The best busy, the next not open: it opens in the background.
            (
                vec![dev("NVIDIA", "o", 1), dev("Intel", "", 0)],
                vec![0, 1],
                Some(1),
            ),
            // A failed device and a software one are never chosen.
            (
                vec![dev("NVIDIA", "of", 0), dev("Intel", "o", 0)],
                vec![1],
                None,
            ),
            (
                vec![dev("NVIDIA", "o", 4), dev("Microsoft", "os", 0)],
                vec![0],
                None,
            ),
            // None left: the CPU.
            (vec![dev("Microsoft", "s", 0)], vec![], None),
            (vec![], vec![], None),
        ];
        for (seen, order, later) in table {
            assert_eq!(least_busy(&seen), (order, later), "{seen:?}");
        }
    }

    #[test]
    fn a_chosen_device_comes_first_then_the_others() {
        let seen = [
            dev("NVIDIA", "", 0),
            dev("AMD", "f", 0),
            dev("Intel", "", 0),
            dev("Microsoft", "s", 0),
        ];
        let table = [
            (0, vec![0, 2]),
            (2, vec![2, 0]),
            (3, vec![3, 0, 2]),
            (1, vec![1, 0, 2]),
        ];
        for (n, want) in table {
            assert_eq!(from(n, &seen), want, "chose {n}");
        }
        assert_eq!(from_best(&seen), Some(0));
        assert_eq!(from_best(&seen[1..]), Some(1));
    }

    #[test]
    fn a_kernel_names_its_device_on_a_header_line() {
        let table = [
            ("// work 4\n// gpu 1\nfn main() {}", Some(1)),
            ("// gpu 0\n// work 1\n", Some(0)),
            ("// work 4\nfn f() {}\n// gpu 1\n", None),
            ("// gpu x\n", None),
            ("", None),
        ];
        for (kernel, want) in table {
            assert_eq!(kernel_device(kernel), want, "{kernel:?}");
        }
    }

    #[test]
    fn the_launch_setting_wins_when_it_names_a_device() {
        let some = |n: &'static str, t: &str| Some((n, t.to_string()));
        let table = [
            (Some("amd"), Some("1"), some("KAVACH_GPU", "amd")),
            (Some("on"), Some("1"), some("[run] gpu", "1")),
            (Some("cpu"), Some("nvidia"), some("[run] gpu", "nvidia")),
            (None, Some("cpu"), None),
            (None, Some("auto"), None),
            (Some("2"), None, some("KAVACH_GPU", "2")),
            (None, None, None),
        ];
        for (env, file, want) in table {
            assert_eq!(setting_text(env, file), want, "{env:?} {file:?}");
        }
    }

    #[test]
    fn a_setting_names_a_device_by_number_or_vendor() {
        let seen = [
            dev("NVIDIA", "", 0),
            dev("Intel", "", 0),
            dev("Microsoft", "s", 0),
        ];
        let table = [
            ("0", Ok(Some(0))),
            ("2", Ok(Some(2))),
            ("3", Err(Missing::Number(3))),
            ("intel", Ok(Some(1))),
            ("Microsoft", Ok(Some(2))),
            ("amd", Err(Missing::Vendor("AMD"))),
            ("cpu", Ok(None)),
            ("auto", Ok(None)),
            ("fastest", Err(Missing::Unread)),
        ];
        for (text, want) in table {
            assert_eq!(number_of(text, &seen), want, "{text}");
        }
    }

    #[test]
    fn a_missing_device_stops_the_start_saying_what_there_is() {
        let seen = [dev("NVIDIA", "", 0), dev("Intel", "", 0)];
        let names = [
            "RTX (NVIDIA, discrete)".to_string(),
            "UHD (Intel, integrated)".to_string(),
        ];
        let has = "this machine has 2: 0 RTX (NVIDIA, discrete), 1 UHD (Intel, integrated)";
        let place = "`gpu(2) fn shade`";
        // The highest number in the code, `KAVACH_GPU`, `[run] gpu`, and
        // what the start says.
        type Case<'a> = (
            Option<usize>,
            Option<&'a str>,
            Option<&'a str>,
            Result<(), String>,
        );
        let table: Vec<Case> = vec![
            (Some(1), None, None, Ok(())),
            (None, Some("1"), Some("nvidia"), Ok(())),
            (
                Some(2),
                None,
                None,
                Err(format!("this program asks for GPU 2 ({place}), and {has}")),
            ),
            (
                None,
                Some("5"),
                None,
                Err(format!("KAVACH_GPU=5 asks for GPU 5, and {has}")),
            ),
            (
                None,
                Some("amd"),
                None,
                Err(format!(
                    "KAVACH_GPU=amd asks for a GPU made by AMD, and {has}"
                )),
            ),
            (
                None,
                None,
                Some("apple"),
                Err(format!(
                    "[run] gpu = \"apple\" asks for a GPU made by Apple, and {has}"
                )),
            ),
            // The launch's device wins: the file's isn't looked for.
            (None, Some("0"), Some("apple"), Ok(())),
            (
                None,
                Some("on"),
                Some("apple"),
                Err(format!(
                    "[run] gpu = \"apple\" asks for a GPU made by Apple, and {has}"
                )),
            ),
            (
                None,
                Some("fast"),
                None,
                Err(gpu_setting::takes("KAVACH_GPU=fast")),
            ),
        ];
        for (highest, env, file, want) in table {
            assert_eq!(check_start(highest, place, env, file, &seen, &names), want);
        }
        let none = check_start(Some(0), place, None, None, &[], &[]);
        assert_eq!(
            none,
            Err(format!(
                "this program asks for GPU 0 ({place}), and this machine has none"
            ))
        );
    }
}
