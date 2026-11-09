//! Reductions a loop doesn't wait for (spec 15.2, decision 0067): the
//! runtime side of the code `compiler/kir/src/gpu_later.rs` makes. Each
//! round's reduction goes into the GPU's queue, and into a queue here. The
//! program takes the oldest out, with `GpuLaterNext`, only when too much is
//! under way, or the newest must run on the CPU, or the loop has ended; so
//! the next chunk is read and copied over while the GPU works.

use super::reduce::{combine, gave_value, plan};
use super::{bring_back, fetch, message, slot, take_alone, with_kernel_bytes};
use crate::rt::{panic_with, type_of};
use kavach_kir::{RtOp, Type};
use kavach_runtime_gpu::{GpuError, Items, Later};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

/// The most rounds under way, and the most bytes of GPU memory they may
/// take: a round's items and its results. Measured on 2 GB in chunks of
/// 64 MB (benchmarks/gpu), 3 and 4 rounds were no faster than 2.
const ROUNDS: usize = 2;
const BYTES: usize = 1 << 30;

/// A round's reduction.
struct Round {
    list: i64,
    op: RtOp,
    ty: Type,
    line: i32,
    /// The work under way; none when the CPU must do it.
    work: Option<Work>,
}

struct Work {
    later: Later,
    kernel: String,
    /// The list's items in GPU memory, when they left the table (`kept.rs`)
    /// for this round, and whether they are only there.
    items: Option<(kavach_runtime_gpu::Resident, bool)>,
    /// How many `map`s' stop flags came before it.
    flags: u64,
    bytes: usize,
}

thread_local! {
    /// The queues by number, each with something in it.
    static QUEUES: RefCell<Vec<(i64, VecDeque<Round>)>> = const { RefCell::new(Vec::new()) };
    static NEXT: Cell<i64> = const { Cell::new(0) };
    /// The round just made, for its queue.
    static MADE: RefCell<Option<Round>> = const { RefCell::new(None) };
    /// The value and list of the round `GpuLaterNext` gave.
    static GIVEN: Cell<(i64, i64)> = const { Cell::new((0, 0)) };
}

/// `GpuLaterNew`.
pub(super) fn new_queue() -> i64 {
    NEXT.with(|n| n.replace(n.get() + 1))
}

/// `GpuSumLater`, `GpuMinLater` or `GpuMaxLater`.
pub(super) fn add(op: RtOp, list: i64, closure: i64, text: i64, code: i64, line: i32) -> i64 {
    let op = match op {
        RtOp::GpuSumLater => RtOp::GpuSum,
        RtOp::GpuMinLater => RtOp::GpuMin,
        _ => RtOp::GpuMax,
    };
    let p = plan(op, list, closure, text);
    let later = p.wanted.then(|| match &p.on_gpu {
        Some(r) => {
            kavach_runtime_gpu::reduce_later(&p.kernel, Items::Resident(r), p.n, &p.captures)
        }
        None => with_kernel_bytes(list, p.item, p.n, |b| {
            kavach_runtime_gpu::reduce_later(&p.kernel, Items::Bytes(b), p.n, &p.captures)
        }),
    });
    let work = match later {
        Some(Ok(later)) => Some(Work {
            later,
            bytes: 2 * p
                .on_gpu
                .as_ref()
                .map_or(0, kavach_runtime_gpu::Resident::bytes),
            items: take_alone(list),
            flags: kavach_runtime_gpu::flags_so_far(),
            kernel: p.kernel,
        }),
        _ => {
            // The CPU reads the list.
            fetch(list);
            None
        }
    };
    let round = Round {
        list,
        op,
        ty: type_of(code),
        line,
        work,
    };
    MADE.with(|m| *m.borrow_mut() = Some(round));
    0
}

/// `GpuLaterNext`: adds the round just made to queue `q`, and whether one
/// is due: its statement runs now, with what `GpuLaterValue` and
/// `GpuLaterList` give.
pub(super) fn next(q: i64, all: i64) -> i64 {
    let made = MADE.with(|m| m.borrow_mut().take());
    let due = QUEUES.with(|queues| {
        let mut queues = queues.borrow_mut();
        let at = match queues.iter().position(|(id, _)| *id == q) {
            Some(at) => at,
            None if made.is_some() => {
                queues.push((q, VecDeque::new()));
                queues.len() - 1
            }
            None => return None,
        };
        let rounds = &mut queues[at].1;
        rounds.extend(made);
        let bytes: usize = rounds
            .iter()
            .filter_map(|r| r.work.as_ref())
            .map(|w| w.bytes)
            .sum();
        let waits = all != 0
            || rounds.len() > ROUNDS
            || bytes > BYTES
            || rounds.iter().any(|r| r.work.is_none());
        let round = if waits { rounds.pop_front() } else { None };
        if rounds.is_empty() {
            queues.swap_remove(at);
        }
        round
    });
    match due {
        Some(round) => {
            finish(round);
            1
        }
        None => 0,
    }
}

/// Waits for `round`, and leaves its value, or its list on the CPU for the
/// statement to work it out from. A `gpu fn`'s panic comes here.
fn finish(round: Round) {
    let list = round.list;
    let Some(work) = round.work else {
        GIVEN.set((0, list));
        return;
    };
    let res = work.later.wait();
    if let Some((code, a, b, line, kernel)) = kavach_runtime_gpu::finish_before(work.flags) {
        panic_with(&message(code, a, b, &kernel), line as i32)
    }
    let signed = round.ty == Type::I32;
    let value = match res {
        Ok(partials) => combine(round.op, &partials, signed),
        // Code 1: a partial sum overflowed; the CPU adds it in order.
        Err(GpuError::Unavailable | GpuError::Stopped { code: 1, .. }) => None,
        Err(GpuError::Stopped {
            code,
            a,
            b,
            line: at,
        }) => {
            let at = i32::try_from(at)
                .ok()
                .filter(|l| *l > 0)
                .unwrap_or(round.line);
            panic_with(&message(code, a, b, &work.kernel), at)
        }
    };
    match value {
        Some(v) => {
            gave_value();
            GIVEN.set((slot(v, round.ty) as i64, list));
        }
        None => {
            match &work.items {
                Some((items, true)) => bring_back(list, items),
                Some(_) => {}
                None => {
                    fetch(list);
                }
            }
            GIVEN.set((0, list));
        }
    }
}

/// `GpuLaterValue`.
pub(super) fn value() -> i64 {
    GIVEN.get().0
}

/// `GpuLaterList`: the list, handed over.
pub(super) fn list() -> i64 {
    GIVEN.get().1
}
