//! `list.map(f)` on the GPU (spec 15.2, decision 0044): the runtime side
//! of `GpuMap`. It gives the mapped list, or 0 (`none`) when the CPU
//! should run the `map`: no kernel, a short list, no GPU, or
//! `KAVACH_GPU=off`. A kernel's panic becomes the CPU's message.

use crate::display::{f32_of, f32_slot};
use crate::heap::{alloc_slots, items_of, obj, release, shape, slot_types, with_str};
use crate::rt::{panic_with, type_of};
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::format_float;

mod filter;
mod kept;
mod later;
mod reduce;

use kept::{bring_back, fetch, forget, keep, resident, take_alone};

/// The least work worth sending to an open GPU, in nanoseconds of the
/// CPU: below it, starting the GPU costs more than it saves.
const WORTH_NS: u64 = 2_000_000;

/// About how long opening the GPU takes: while it opens, only work that
/// would take the CPU longer waits for it.
const OPENING_NS: u64 = 300_000_000;

/// Whether `n` items of `kernel`'s work are worth the GPU (spec 15.2):
/// the kernel's first line, `// work <n>`, is its work per item.
fn worth_it(kernel: &str, n: usize) -> bool {
    let work = kernel
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("// work "))
        .and_then(|w| w.parse::<u64>().ok())
        .unwrap_or(1);
    // The CPU's time, at about 0.1 ns per unit of work: measured from
    // 0.05 (short loops it vectorizes) to 0.35 (light work, where memory
    // is the limit), with long dependent loops at 0.2.
    let cpu = (n as u64).saturating_mul(work) / 10;
    match kavach_runtime_gpu::ready() {
        true => cpu >= WORTH_NS,
        false => cpu >= OPENING_NS,
    }
}

crate::rt_data::entry!(kv_rt_gpu, |op, a, b, c, code, line| match op {
    RtOp::GpuMap => map(a, b, c, code, line, false),
    RtOp::GpuMapStay => map(a, b, c, code, line, true),
    RtOp::GpuFetch => fetch(a),
    RtOp::GpuSum | RtOp::GpuMin | RtOp::GpuMax => reduce::reduce(op, a, b, c, code, line),
    RtOp::GpuReduced => reduce::reduced(),
    RtOp::GpuSumLater | RtOp::GpuMinLater | RtOp::GpuMaxLater => {
        later::add(op, a, b, c, code, line)
    }
    RtOp::GpuLaterNew => later::new_queue(),
    RtOp::GpuLaterNext => later::next(a, b),
    RtOp::GpuLaterValue => later::value(),
    RtOp::GpuLaterList => later::list(),
    RtOp::GpuFilter => filter::filter(a, b, c, code, line, false),
    RtOp::GpuFilterStay => filter::filter(a, b, c, code, line, true),
    RtOp::GpuCheck => check(a, line),
    RtOp::GpuWarm => warm(a),
    _ => 0,
});

thread_local! {
    /// With `KAVACH_GPU=check`: the GPU's list, and its item type, waiting
    /// for the CPU's to compare with.
    static PENDING: std::cell::RefCell<Option<(Vec<u64>, Type, usize)>> = const { std::cell::RefCell::new(None) };
}

/// Panics for the first item that stopped in a `map` that didn't wait
/// for the GPU (decision 0061), now that the program reads what it made,
/// waits for the GPU, or ends.
fn finish_late() {
    if let Some((code, a, b, line, kernel)) = kavach_runtime_gpu::finish() {
        panic_with(&message(code, a, b, &kernel), line as i32)
    }
}

/// `KAVACH_GPU=check` (spec 15.2): the CPU's `list` against the GPU's,
/// floats compared with `near`; gives `list`.
fn check(list: i64, line: i32) -> i64 {
    let Some((gpu, t, width)) = PENDING.with(|p| p.borrow_mut().take()) else {
        return list;
    };
    // Records and tuples are compared field by field, in the flat list's
    // words; other items one by one.
    let cpu: Vec<u64> = match width {
        1 => items_of(list).iter().copied().collect(),
        w => {
            let words = crate::heap::list_len_items_pub(list) * w;
            crate::heap::slots(list)[..words].to_vec()
        }
    };
    let type_at = |j: usize| match width {
        1 => t,
        w => crate::flat::field_type(t, j % w),
    };
    let same = |j: usize, a: u64, b: u64| match type_at(j) {
        Type::F32 => {
            let (x, y) = (f32_of(a as i64), f32_of(b as i64));
            kavach_runtime_core::float32_near(x, y) || (x.is_nan() && y.is_nan())
        }
        _ => a == b,
    };
    let differ = (cpu.iter().zip(&gpu).enumerate()).position(|(j, (c, g))| !same(j, *c, *g));
    if let Some(j) = differ {
        let (i, t) = (j / width, type_at(j));
        let show = |s: u64| crate::display::display(s as i64, t, false);
        let (c, g) = (show(cpu[j]), show(gpu[j]));
        drop(cpu);
        panic_with(
            &format!("KAVACH_GPU=check: item {i} is {g} on the GPU and {c} on the CPU"),
            line,
        );
    }
    list
}

/// Opens the GPU and compiles `kernels` (joined by `\0`) on a background
/// thread; a `map` that comes first waits for it.
fn warm(kernels: i64) -> i64 {
    let text = with_str(kernels, str::to_string);
    release(kernels);
    if std::env::var("KAVACH_GPU").as_deref() == Ok("off") {
        return 0;
    }
    std::thread::spawn(move || {
        for kernel in text.split('\0') {
            kavach_runtime_gpu::prepare(kernel);
        }
    });
    0
}

/// The values the kernel's closures captured, as `info` holds them after
/// the item count (`compiler/backend/wgsl`): field `k` of the closure is
/// `info[k]`, and field 0, the function, is never read. For a chain of
/// `map`s (a pair of closures), the second's follow the first's.
fn captures_of(closure: i64) -> Vec<u32> {
    let one = |c: i64| -> Vec<u32> {
        let fields = items_of(c);
        let words: Vec<u32> = fields
            .iter()
            .zip(slot_types(c))
            .map(|(s, t)| word(*s, t))
            .skip(1)
            .collect();
        words
    };
    match shape(obj(closure).ty()) {
        Shape::Tuple(_) => items_of(closure)
            .iter()
            .flat_map(|c| one(*c as i64))
            .collect(),
        _ => one(closure),
    }
}

/// Slot `s` of type `t` as one word for a kernel.
fn word(s: u64, t: Type) -> u32 {
    match t {
        Type::F32 => f32_of(s as i64).to_bits(),
        _ => s as u32,
    }
}

/// Kernel word `w` as a slot of type `t`.
fn slot(w: u32, t: Type) -> u64 {
    match t {
        Type::I32 => w as i32 as i64 as u64,
        Type::F32 => f32_slot(f32::from_bits(w)) as u64,
        Type::Bool => u64::from(w != 0),
        _ => u64::from(w),
    }
}

/// `words` as their bytes, in memory order (little-endian).
pub(super) fn as_bytes(words: &[u64]) -> &[u8] {
    // SAFETY: any `u64` is 8 valid bytes, `u8` needs no alignment, and the
    // slice borrows `words` for its whole life.
    unsafe { std::slice::from_raw_parts(words.as_ptr().cast(), words.len() * 8) }
}

/// `words` as their bytes, to write.
fn as_bytes_mut(words: &mut [u64]) -> &mut [u8] {
    // SAFETY: as in `as_bytes`, and any 8 bytes are a valid `u64`.
    unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast(), words.len() * 8) }
}

/// The `n` items of `list`, of type `item`, as a kernel's 32-bit words
/// (little-endian), given to `f`: a packed list of 32-bit items
/// (decision 0045) is in that form already, and isn't copied.
fn with_kernel_bytes<R>(list: i64, item: Type, n: usize, f: impl FnOnce(&[u8]) -> R) -> R {
    let ty = obj(list).ty();
    if crate::flat::item_size(ty) == 4 {
        return f(&as_bytes(crate::heap::slots(list))[..n * 4]);
    }
    // A flat list of tuples or records: their fields, one word each, item
    // after item, as the kernel reads them.
    if let (w @ 1.., false) = (crate::flat::width(ty), crate::flat::is_text(ty)) {
        let bytes: Vec<u8> = crate::heap::slots(list)[..n * w]
            .iter()
            .enumerate()
            .flat_map(|(j, s)| word(*s, crate::flat::field_type(item, j % w)).to_le_bytes())
            .collect();
        return f(&bytes);
    }
    let bytes: Vec<u8> = items_of(list)
        .iter()
        .flat_map(|s| word(*s, item).to_le_bytes())
        .collect();
    f(&bytes)
}

/// `stay`: the result may stay in GPU memory only, as every read of it
/// fetches it first.
fn map(list: i64, closure: i64, text: i64, code: i64, line: i32, stay: bool) -> i64 {
    let kernel = with_str(text, str::to_string);
    release(text);
    let n = crate::heap::list_len_items_pub(list);
    let mode = std::env::var("KAVACH_GPU").unwrap_or_default();
    let checking = mode == "check";
    let wanted = mode == "on" || checking || (mode != "off" && worth_it(&kernel, n));
    // An empty list has nothing to send: a GPU buffer can't be empty.
    if kernel.is_empty() || !wanted || n == 0 {
        // The CPU reads it next.
        fetch(list);
        release(list);
        release(closure);
        return 0;
    }
    let item = match shape(obj(list).ty()) {
        Shape::List(t) => *t,
        _ => Type::Unit,
    };
    let captures = captures_of(closure);
    release(closure);
    let captures = &captures[..];
    let list_ty = match shape(type_of(code)) {
        Shape::Optional(t) => *t,
        _ => type_of(code),
    };
    let result = match shape(list_ty) {
        Shape::List(t) => *t,
        _ => Type::Unit,
    };
    // A packed result (decision 0045) takes the kernel's words straight
    // into its buffer; any other is made from them. A record or tuple
    // result is `width` words per item (the kernel's `// out` line).
    let width = kavach_runtime_gpu::out_width(&kernel);
    let direct = !checking && width == 1 && crate::flat::item_size(list_ty) == 4;
    // A flat list of records stays too: its fields come back on `fetch`.
    let stay = stay && (direct || (!checking && width > 1));
    let mut out = vec![0u64; (n * width).div_ceil(2)];
    let bytes = &mut as_bytes_mut(&mut out)[..n * width * 4];
    // The list's holders are this call, the `map`'s own copy of it
    // (`lower_gpu.rs`: the local it is held in, or `ps` of `ps =
    // ps.map(f)`, which the result replaces) and `keep`. With no other,
    // nothing can read its items again, so the GPU writes its results
    // over them.
    let over = stay && obj(list).rc == 3;
    let res = match resident(list) {
        Some(on_gpu) => {
            let items = kavach_runtime_gpu::Items::Resident(&on_gpu);
            kavach_runtime_gpu::map(&kernel, items, captures, bytes, stay, over)
        }
        None => with_kernel_bytes(list, item, n, |items| {
            let items = kavach_runtime_gpu::Items::Bytes(items);
            kavach_runtime_gpu::map(&kernel, items, captures, bytes, stay, false)
        }),
    };
    if over && res.is_ok() {
        forget(list);
    }
    if !stay {
        finish_late();
    }
    if let Ok(kavach_runtime_gpu::Kept {
        items: Some(on_gpu),
        ..
    }) = &res
    {
        keep(list, on_gpu.clone(), false);
    }
    if res.is_err() {
        fetch(list);
    }
    release(list);
    match res {
        Ok(kept) if !checking => {
            let made = match direct {
                true => crate::heap::new_list_raw(list_ty, 0, out, n),
                // A flat list of records or tuples: their fields, slot by
                // slot; when they stay on the GPU, `fetch` writes them.
                false if stay => crate::heap::alloc_words(list_ty, vec![0; n * width]),
                false if width > 1 => crate::heap::alloc_words(
                    list_ty,
                    (0..n * width)
                        .map(|j| {
                            let w = crate::flat::get_item(&out, j, 4) as u32;
                            slot(w, crate::flat::field_type(result, j % width))
                        })
                        .collect(),
                ),
                false => {
                    let slots =
                        (0..n).map(|i| slot(crate::flat::get_item(&out, i, 4) as u32, result));
                    alloc_slots(list_ty, 0, slots.collect())
                }
            };
            if let Some(on_gpu) = kept.results {
                keep(made, on_gpu, stay);
            }
            made
        }
        Ok(_) => {
            // The CPU runs it too, and `check` compares the two.
            let type_at = |j: usize| match width {
                1 => result,
                w => crate::flat::field_type(result, j % w),
            };
            let slots: Vec<u64> = (0..n * width)
                .map(|j| slot(crate::flat::get_item(&out, j, 4) as u32, type_at(j)))
                .collect();
            PENDING.with(|p| *p.borrow_mut() = Some((slots, result, width)));
            0
        }
        Err(kavach_runtime_gpu::GpuError::Unavailable) => 0,
        // The line inside the `gpu fn` where the item stopped, as on the
        // CPU; the `map`'s when the kernel didn't say.
        Err(kavach_runtime_gpu::GpuError::Stopped {
            code,
            a,
            b,
            line: at,
        }) => {
            let at = i32::try_from(at).ok().filter(|l| *l > 0).unwrap_or(line);
            panic_with(&message(code, a, b, &kernel), at)
        }
    }
}

/// The CPU's message for what stopped an item (codes: `compiler/backend/wgsl`).
fn message(code: u32, a: u32, b: u32, kernel: &str) -> String {
    let int = |t: u32, w: u32| {
        if t == 1 {
            (w as i32).to_string()
        } else {
            w.to_string()
        }
    };
    let name = |t: u32| match t {
        1 => "Int32",
        2 => "UInt32",
        _ => "Float32",
    };
    match code {
        20 => "division by zero".to_string(),
        91 => format!("integer overflow: -({}) doesn't fit in Int32", a as i32),
        11..=82 => {
            let op = ["+", "-", "*", "/", "%", "**", "<<", ">>"][(code / 10 - 1) as usize];
            let t = code % 10;
            format!(
                "integer overflow: {} {op} {} doesn't fit in {}",
                int(t, a),
                int(t, b),
                name(t)
            )
        }
        300..=399 => {
            let (from, to) = ((code / 10) % 10, code % 10);
            let value = match from {
                3 => format_float(f64::from(f32::from_bits(a))),
                t => int(t, a),
            };
            format!("{value} doesn't fit in {}", name(to))
        }
        _ => kernel
            .lines()
            .find_map(|l| l.strip_prefix(&format!("// stop {code}: ")))
            .unwrap_or("a gpu fn panicked")
            .to_string(),
    }
}
