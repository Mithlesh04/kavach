// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The program's GPUs (spec 15.6): one table, listed once at the first
//! GPU work, so a device's number never changes while the program runs.
//! A device is opened when work first goes to it; one that can't be
//! opened, or is lost, is skipped from then on. Which device gets the
//! work is worked out by the plain functions of `choice.rs`.

use crate::choice::{self, Seen};
use crate::device::{self, Gpu};
use crate::listing::{self, Kind, Listed, Real};
use crate::split::{self, Mode};
use kavach_runtime_core::GpuPlace;
use std::ops::Range;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// One device of the table.
pub(crate) struct Device {
    pub(crate) listed: Listed,
    /// How to open it: the machine's GPU it is, if there is one (a fake
    /// entry of `KAVACH_FAKE_GPUS` opens the real GPU of its name).
    source: OnceLock<Option<(wgpu::Instance, wgpu::Adapter)>>,
    opened: OnceLock<Option<Gpu>>,
    /// Can't be opened, or lost: skipped.
    failed: AtomicBool,
    /// Being opened in the background, for when its betters are busy.
    opening: AtomicBool,
    /// Its measured speed on parts of split maps, work per nanosecond
    /// (an `f64`'s bits; 0 before its first part).
    speed: AtomicU64,
}

static TABLE: OnceLock<Vec<Device>> = OnceLock::new();

/// Every GPU the program could use, best first (spec 14.25, 15.6).
pub(crate) fn table() -> &'static [Device] {
    TABLE.get_or_init(|| {
        let fake = std::env::var("KAVACH_FAKE_GPUS").ok();
        let listed: Vec<(Listed, Option<(wgpu::Instance, wgpu::Adapter)>)> = match fake {
            Some(text) => (listing::fake(&text).into_iter())
                .map(|l| (l, None))
                .collect(),
            None => (listing::real().into_iter())
                .map(|(l, i, a): Real| (l, Some((i, a))))
                .collect(),
        };
        (listing::ranked(listed, |(l, _)| l).into_iter())
            .map(|(listed, source)| {
                // A fake finds its GPU only when it is opened.
                let found = OnceLock::new();
                if source.is_some() {
                    let _ = found.set(source);
                }
                Device {
                    listed,
                    source: found,
                    opened: OnceLock::new(),
                    failed: AtomicBool::new(false),
                    opening: AtomicBool::new(false),
                    speed: AtomicU64::new(0),
                }
            })
            .collect()
    })
}

/// The machine's GPU named `name`, for a fake entry.
fn real_named(name: &str) -> Option<(wgpu::Instance, wgpu::Adapter)> {
    static REAL: OnceLock<Vec<Real>> = OnceLock::new();
    let real = REAL.get_or_init(listing::real);
    (real.iter())
        .find(|(l, ..)| l.name == name)
        .map(|(_, i, a)| (i.clone(), a.clone()))
}

/// What `choice.rs` sees of each device now.
fn seen() -> Vec<Seen> {
    (table().iter())
        .map(|d| {
            let open = d.opened.get().and_then(Option::as_ref);
            Seen {
                vendor: listing::vendor(d.listed.vendor),
                software: d.listed.kind == Kind::Software,
                failed: d.failed.load(Ordering::Relaxed) || open.is_some_and(Gpu::is_lost),
                open: open.is_some(),
                busy: open.map_or(0, |g| g.busy.load(Ordering::Relaxed)),
            }
        })
        .collect()
}

/// Device `i`, opened now if it isn't yet; `None` when it can't be.
pub(crate) fn device(i: usize) -> Option<&'static Gpu> {
    let d = table().get(i)?;
    if d.failed.load(Ordering::Relaxed) {
        return None;
    }
    let gpu = d.opened.get_or_init(|| {
        let source = d.source.get_or_init(|| real_named(&d.listed.name)).clone();
        let opened = match source {
            Some((instance, adapter)) => device::open(i, instance, adapter),
            None => Err("this machine has no GPU of that name".into()),
        };
        note(i, opened.as_ref().err().map(String::as_str));
        opened.ok()
    });
    match gpu {
        Some(g) if !g.is_lost() => Some(g),
        _ => {
            if !d.failed.swap(true, Ordering::Relaxed) && gpu.is_some() {
                note(i, Some("it was lost or ran out of memory"));
            }
            None
        }
    }
}

/// With `KAVACH_PAR_LOG`, notes device `i` opened, or skipped and why.
fn note(i: usize, why_not: Option<&str>) {
    if std::env::var_os("KAVACH_PAR_LOG").is_none() {
        return;
    }
    let l = &table()[i].listed;
    match why_not {
        None => eprintln!("kavach: GPU {i} ({}, {}) opened", l.name, l.backend),
        Some(why) => eprintln!("kavach: GPU {i} ({}) skipped: {why}", l.name),
    }
}

/// The device `KAVACH_GPU` or `[run] gpu` names, if one: what the
/// system's choice gives way to. Read once; one that isn't there was
/// reported at the start (`start`).
fn setting() -> Option<usize> {
    static SETTING: OnceLock<Option<usize>> = OnceLock::new();
    *SETTING.get_or_init(|| {
        let text = choice::setting_text(
            std::env::var("KAVACH_GPU").ok().as_deref(),
            kavach_runtime_core::file_gpu(),
        )?;
        choice::number_of(&text.1, &seen()).ok().flatten()
    })
}

/// The device the code's choice, or else the setting, asks for:
/// `gpu.on`, `on gpu(n)` or `on:` around the work, then the kernel's
/// `gpu(n) fn` (its `// gpu n` line), then `KAVACH_GPU` or `[run] gpu`.
fn asked(kernel: &str) -> Option<usize> {
    match kavach_runtime_core::gpu_place() {
        GpuPlace::Device(n) => Some(n),
        _ => choice::kernel_device(kernel).or_else(setting),
    }
}

/// The first of `order` that opens.
fn first_open(order: Vec<usize>) -> Option<&'static Gpu> {
    order.into_iter().find_map(device)
}

/// The device for GPU work of `kernel` now (spec 15.6), or `None` for the
/// CPU. `holder` is the device the items are already on, if any.
pub(crate) fn pick(kernel: &str, holder: Option<usize>) -> Option<&'static Gpu> {
    let seen = seen();
    if let Some(n) = asked(kernel) {
        return first_open(choice::from(n, &seen));
    }
    if let Some(h) = holder.filter(|h| !seen[*h].failed)
        && let Some(g) = device(h)
    {
        return Some(g);
    }
    let (best, later) = choice::least_busy(&seen);
    if let Some(i) = later {
        open_later(i);
    }
    first_open(best)
}

/// Opens device `i` on a thread of its own, once: work goes to it when
/// it is ready, and doesn't wait for it.
fn open_later(i: usize) {
    if !table()[i].opening.swap(true, Ordering::Relaxed) {
        std::thread::spawn(move || {
            device(i);
        });
    }
}

/// How a `map` of kernel `kernel` over `n` items in main memory, of
/// `words` 32-bit words of item and result each, splits across the GPUs
/// (spec 15.6): each device's number and its run of items, in order.
/// `None` when it doesn't split: a device is chosen, fewer than two
/// devices are open, or its size doesn't pay. A device not open yet
/// opens in the background for later work, or now with
/// `KAVACH_GPU_SPLIT=on`.
pub(crate) fn split(kernel: &str, n: usize, words: usize) -> Option<Vec<(usize, Range<usize>)>> {
    static MODE: OnceLock<Mode> = OnceLock::new();
    let mode = *MODE.get_or_init(|| Mode::of(std::env::var("KAVACH_GPU_SPLIT").ok().as_deref()));
    let work = split::work_per_item(kernel);
    if !split::pays(mode, n, work, words) || asked(kernel).is_some() {
        return None;
    }
    let seen = seen();
    let mut taking = Vec::new();
    for i in choice::usable(&seen) {
        match (seen[i].open, mode) {
            (true, _) => taking.push(i),
            (false, Mode::Always) => {
                if device(i).is_some() {
                    taking.push(i);
                }
            }
            (false, _) => open_later(i),
        }
    }
    let each: Vec<(f64, f64)> = (taking.iter())
        .map(|&i| {
            let d = &table()[i];
            let measured = f64::from_bits(d.speed.load(Ordering::Relaxed));
            (measured, split::guess(d.listed.kind))
        })
        .collect();
    let speeds = split::speeds(&each);
    let parts = split::parts(n, &speeds)?;
    // Once every device has a measured speed, a split must beat the
    // fastest alone.
    let measured = each.iter().all(|(m, _)| *m > 0.0);
    if mode == Mode::Auto && measured && !split::beats_one(n, work, &speeds, &parts) {
        return None;
    }
    let parts: Vec<(usize, Range<usize>)> = (taking.into_iter().zip(parts))
        .filter_map(|(i, run)| run.map(|r| (i, r)))
        .collect();
    if std::env::var_os("KAVACH_PAR_LOG").is_some() {
        let each: Vec<String> = (parts.iter())
            .map(|(i, r)| format!("{i} ({})", r.len()))
            .collect();
        eprintln!(
            "kavach: a map of {n} items split across GPUs: {}",
            each.join(", ")
        );
    }
    Some(parts)
}

/// Device `i` did `work` units in `ns` nanoseconds of a split map.
pub(crate) fn measured(i: usize, work: u64, ns: u128) {
    if ns == 0 {
        return;
    }
    let speed = &table()[i].speed;
    let last = f64::from_bits(speed.load(Ordering::Relaxed));
    let now = split::averaged(last, work as f64 / ns as f64);
    speed.store(now.to_bits(), Ordering::Relaxed);
}

/// The program's GPU, for drawing and for compiling kernels ahead: the
/// device `KAVACH_GPU` or `[run] gpu` names, or else the best.
pub(crate) fn gpu() -> Option<&'static Gpu> {
    let seen = seen();
    match setting() {
        Some(n) => first_open(choice::from(n, &seen)),
        None => first_open(choice::least_busy(&seen).0),
    }
}

/// Whether this machine has a GPU `map` can use.
pub fn available() -> bool {
    gpu().is_some()
}

/// Whether the program's GPU is open and ready now, without waiting.
pub fn ready() -> bool {
    let Some(table) = TABLE.get() else {
        return false;
    };
    let main = setting().or_else(|| choice::from_best(&seen()));
    main.and_then(|i| table.get(i))
        .is_some_and(|d| matches!(d.opened.get(), Some(Some(_))))
}

/// At the program's start (spec 15.6): whether every device it asks for
/// is there, `highest` the highest number written in the code (`where`
/// saying where), and `file` the text of `[run] gpu`. `Err` says why
/// not, naming the devices the machine has.
pub fn start(highest: Option<usize>, place: &str, file: Option<&str>) -> Result<(), String> {
    if let Some(text) = file {
        kavach_runtime_core::set_file_gpu(text);
    }
    let names: Vec<String> = (table().iter())
        .map(|d| {
            let [name, vendor, kind, _] = d.listed.info();
            format!("{name} ({vendor}, {})", kind.to_lowercase())
        })
        .collect();
    let env = std::env::var("KAVACH_GPU").ok();
    choice::check_start(highest, place, env.as_deref(), file, &seen(), &names)
}

/// The panic of choosing device `n` while running, when the machine has
/// no such device (spec 15.6).
pub fn missing(n: i64) -> Option<String> {
    let k = table().len();
    let gone = usize::try_from(n).map_or(true, |n| n >= k);
    gone.then(|| format!("there is no GPU {n}: this machine has {k}, numbered from 0 (spec 15.6)"))
}

/// At the start of a program with GPU work: `KAVACH_GPU` naming a device
/// that isn't there, or that can't be read, is `Err` with why. The list
/// is read only when it names a device.
pub fn check_launch() -> Result<(), String> {
    match std::env::var("KAVACH_GPU") {
        Ok(text) if choice::setting_text(Some(&text), None).is_some() => start(None, "", None),
        Ok(text) if kavach_runtime_core::gpu_setting::read(&text).is_none() => {
            start(None, "", None)
        }
        _ => Ok(()),
    }
}
