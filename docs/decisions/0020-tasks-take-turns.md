# 0020. Tasks take turns, in a defined order, before they use all cores

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

Until now `spawn` ran its task at once, to the end, in the interpreter and in native code alike. Spec chapter 9 asks for more: tasks that run at the same time, a task that can't outlive its function, errors that come out of `.wait()`.

Three facts constrain how:

- **Native code counts references without atomics.** A copy of a value is the same object with its count raised by one. Two threads that touch one object at the same time would corrupt the count. Atomic counts everywhere would slow every program, with or without tasks.
- **Native code reads two globals directly:** the allocation pool (`kv_pool`) and the stack limit (`kv_stack_limit`). Per-thread copies need thread-local storage in code from both backends.
- **The interpreter is the reference** (decision 0014). Its values are `Rc`, and it evaluates by recursion on the Rust stack with `&mut self`, so a task can't be paused in the middle with safe Rust, and its values can't move to another thread.

And a goal: a program prints the same every time, in development and in a release build. Tests compare the interpreter's output with native code's, line by line.

## Decision

Tasks come in two stages.

**Stage 1 (now): tasks take turns.** Each task is an OS thread, and a baton lets exactly one run at a time. A task hands the baton on only when it waits (spec 9.7). The order of turns is defined, so output is the same on every run and in both implementations. Waiting overlaps: two tasks sleeping 100 ms take 100 ms. One scheduler in `runtime/core/src/tasks.rs` decides the turns for both the interpreter and native code.

Because only the baton holder runs, and handing it on goes through a mutex (which orders memory between threads), native code keeps its plain reference counts, its global pool and its global stack limit (set again at each handoff). The interpreter moves values between task threads only while holding the baton, through one wrapper type that is `Send`: the only `unsafe` outside `runtime/native` (decision 0016's rule), with its reason written next to it.

A function that spawned tasks waits for those still running before it returns (spec 9.2). Cancelling them instead comes with timeouts (to-do item 3).

**Stage 2 (next): all cores.** Tasks that run at the same time need values that one thread owns. Since values are copied into a task anyway (spec 9.5), a task will get its own heap, and values cross between tasks as copies; `Shared<T>` becomes the one value with an atomic count and a lock. The order of turns in 9.7 then holds for what tasks print and send, not for when they compute.

## Consequences

- Programs with tasks run correctly now, with the same output everywhere, and overlap their waiting: the common case for tasks (several requests, timers).
- CPU-heavy work does not get faster with tasks yet. That is Stage 2, which the speed targets (to-do item 1) need.
- A task costs an OS thread for now (tens of microseconds to start, its own stack). Tens of thousands of tasks at once are too many until tasks become lighter in Stage 2.
- The scheduler is shared code, so the interpreter and native code can't drift apart in the order they run tasks.

## Alternatives considered

- **Atomic reference counts, real threads now.** Every program pays for every copy, and the interpreter would need `Arc` everywhere. Rejected: speed is Kavach's first promise.
- **Stackful coroutines (one thread, switching stacks).** Lighter than threads, but the interpreter can't use them with safe Rust (its `&mut self` would be shared between coroutines), and native code would need its own stack switching per platform. Threads with a baton give the same turns with standard tools; coroutines can replace them later without changing any program.
- **Running other tasks inside a waiting task** (no thread, no switching). Fails for producers and consumers that take turns: the waiting task stays under the one it started, so it can't run again until that one ends.
- **No defined order.** Simpler, but output would change from run to run and between development and release builds, and tests couldn't compare them.
