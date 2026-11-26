# 0146. One `unsafe` hand-off between the interpreter's task threads

- **Status:** Accepted (for a maintainer's review)
- **Date:** 2026-11-26

## Context

The interpreter runs each task on a thread of its own, and only one task runs at a time: the task holding the turn (`compiler/interp/src/tasks.rs`). A task starts with a copy of the spawning task's frame and the program's tables, and gives back a result. These values are full of `Rc`s, whose counts are not atomic, so Rust doesn't let them cross threads.

The interpreter has always moved them across with `unsafe impl<T> Send for Baton<T>`. That was Rust `unsafe` outside the places AGENTS.md section 8 allows, with no decision record. The `unsafe` audit ([unsafe-audit.md](../architecture/unsafe-audit.md), finding (a) 5) found it, and found a real bug behind it: a task's last handle could be let go of after its thread gave up the turn, changing `Rc` counts while another task ran.

## Decision

The interpreter keeps one `unsafe` hand-off, in `compiler/interp/src/baton.rs` only:

1. `unsafe impl<T: Crosses> Send for Baton<T>`, where `Crosses` is a private marker trait implemented for exactly four kinds: the task's end (`Flow`), the `Shared` values it made, its result (`Eval`), and the spawn job. Each impl's comment names where that kind is made, read and let go of.
2. Every value a task holds is let go of before its turn passes. `run_task` and the main task drop their handles, frames and task tables before `end_turn`.
3. A debug check guards it: `end_turn` marks the thread as having given up its turn, and `Drop for TaskHandle` asserts the mark isn't set. A handle let go of off the turn is a clear panic in tests, not silent corruption.

AGENTS.md section 8 names `compiler/interp/src/baton.rs` as an allowed place.

## Consequences

- The bug the audit found is fixed, with a test (`tasks_let_their_values_go_before_the_turn_passes`) that failed before the fix.
- The `unsafe` is bounded: adding a new kind of value that crosses threads means implementing `Crosses` in `baton.rs`, which a review sees.
- The argument rests on the turn: only the task holding it touches these values. It must be checked again if the interpreter ever runs tasks at the same time, as native code does (decision 0052).

## Alternatives considered

- **No `unsafe`: values with atomic counts (`Arc`).** Every value in the interpreter would pay for atomic counting, and every `Rc` in it would change. Rejected for now: a large redesign of an interpreter whose future is open (the compiler review's rewrite R3 asks whether it should run KIR instead).
- **Copying values into a thread-safe form at every spawn and wait.** Correct, but it copies the whole frame on each `spawn`, which servers do thousands of times. Rejected for its cost.
- **Running all tasks on one thread.** Removes the hand-off, but a task waiting on input would block the others, which the interpreter's tasks exist to avoid.
