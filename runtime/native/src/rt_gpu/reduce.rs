//! `sum`, `min` and `max` of `Int32` or `UInt32` on the GPU (spec 15.2):
//! the runtime side of `GpuSum`, `GpuMin` and `GpuMax`. They run only on
//! a list already in GPU memory (unless `KAVACH_GPU=on`): copying a list
//! over costs more than the CPU's pass over it. The kernel gives one
//! partial per workgroup, which are combined here in order. A partial
//! sum that overflowed means the CPU adds the list again, in order, so
//! an overflow panics at the right item; `GpuReduced` tells it.

use super::{captures_of, fetch, message, resident, slot, with_kernel_bytes};
use crate::heap::{obj, release, shape, with_str};
use crate::rt::{panic_with, type_of};
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_gpu::{GpuError, Items};
use std::cell::Cell;

thread_local! {
    /// Whether the last reduction gave its value.
    static REDUCED: Cell<bool> = const { Cell::new(false) };
}

/// `GpuReduced`: whether the last `GpuSum`, `GpuMin` or `GpuMax` gave its
/// value, or the CPU must work it out.
pub(super) fn reduced() -> i64 {
    i64::from(REDUCED.with(|r| r.replace(false)))
}

/// Says the reduction now finished gave its value.
pub(super) fn gave_value() {
    REDUCED.with(|r| r.set(true));
}

/// What a reduction needs to run.
pub(super) struct Plan {
    pub(super) kernel: String,
    pub(super) n: usize,
    /// Whether it runs on the GPU, not the CPU.
    pub(super) wanted: bool,
    pub(super) on_gpu: Option<kavach_runtime_gpu::Resident>,
    pub(super) captures: Vec<u32>,
    pub(super) item: Type,
}

/// Decides where a reduction of `list` runs, using up `closure` and `text`.
pub(super) fn plan(op: RtOp, list: i64, closure: i64, text: i64) -> Plan {
    let kernel = with_str(text, str::to_string);
    release(text);
    let n = crate::heap::list_len_items_pub(list);
    let mode = std::env::var("KAVACH_GPU").unwrap_or_default();
    let on_gpu = resident(list);
    let wanted = !kernel.is_empty()
        && n > 0
        && !matches!(mode.as_str(), "off" | "check")
        && (on_gpu.is_some() || mode == "on");
    let captures = match (op, wanted) {
        (RtOp::GpuSum, true) => captures_of(closure),
        _ => Vec::new(),
    };
    if op == RtOp::GpuSum {
        release(closure);
    }
    let item = match shape(obj(list).ty()) {
        Shape::List(t) => *t,
        _ => Type::Unit,
    };
    Plan {
        kernel,
        n,
        wanted,
        on_gpu,
        captures,
        item,
    }
}

pub(super) fn reduce(op: RtOp, list: i64, closure: i64, text: i64, code: i64, line: i32) -> i64 {
    REDUCED.with(|r| r.set(false));
    let Plan {
        kernel,
        n,
        wanted,
        on_gpu,
        captures,
        item,
    } = plan(op, list, closure, text);
    // The CPU reads the list unless the GPU gives the value.
    let cpu = |list: i64| {
        fetch(list);
        release(list);
        0
    };
    if !wanted {
        return cpu(list);
    }
    let res = match on_gpu {
        Some(r) => kavach_runtime_gpu::reduce(&kernel, Items::Resident(&r), n, &captures),
        None => with_kernel_bytes(list, item, n, |b| {
            kavach_runtime_gpu::reduce(&kernel, Items::Bytes(b), n, &captures)
        }),
    };
    // It waited for the GPU: an earlier `map` that didn't is done too.
    super::finish_late();
    let value_ty = type_of(code);
    match res {
        Ok(partials) => match combine(op, &partials, value_ty == Type::I32) {
            Some(v) => {
                gave_value();
                release(list);
                slot(v, value_ty) as i64
            }
            None => cpu(list),
        },
        // Code 1: a partial sum overflowed; the CPU adds it in order.
        Err(GpuError::Unavailable | GpuError::Stopped { code: 1, .. }) => cpu(list),
        Err(GpuError::Stopped {
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

/// The partials, three words for each workgroup, combined in order: the
/// value as a word, or `None` when a running sum of the whole list, as
/// the CPU adds it, leaves the type.
pub(super) fn combine(op: RtOp, partials: &[u32], signed: bool) -> Option<u32> {
    let value = |w: u32| {
        if signed {
            i64::from(w as i32)
        } else {
            i64::from(w)
        }
    };
    let groups = partials.as_chunks::<3>().0.iter();
    match op {
        RtOp::GpuSum => {
            let (lowest, highest) = match signed {
                true => (i64::from(i32::MIN), i64::from(i32::MAX)),
                false => (0, i64::from(u32::MAX)),
            };
            let (mut sum, mut low, mut high) = (0i64, 0i64, 0i64);
            for p in groups {
                low = low.min(sum + value(p[1]));
                high = high.max(sum + value(p[2]));
                sum += value(p[0]);
            }
            (low >= lowest && high <= highest).then_some(sum as u32)
        }
        RtOp::GpuMin => groups.map(|p| value(p[0])).min().map(|v| v as u32),
        _ => groups.map(|p| value(p[0])).max().map(|v| v as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::combine;
    use kavach_kir::RtOp;

    #[test]
    fn partials_combine_in_order() {
        // Two groups: 5 then -3, each with its lowest and highest running sum.
        let words = [5, 0, 7, (-3i32) as u32, (-4i32) as u32, 0];
        assert_eq!(combine(RtOp::GpuSum, &words, true), Some(2));
        // The second group's running sums, after the first's, pass the top.
        let words = [i32::MAX as u32, 0, i32::MAX as u32, 0, 0, 1];
        assert_eq!(combine(RtOp::GpuSum, &words, true), None);
        let words = [3, 0, 0, 9, 0, 0, 2, 0, 0];
        assert_eq!(combine(RtOp::GpuMin, &words, false), Some(2));
        assert_eq!(combine(RtOp::GpuMax, &words, false), Some(9));
    }
}
