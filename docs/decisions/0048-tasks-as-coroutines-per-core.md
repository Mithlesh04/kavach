# 0048. Tasks as coroutines, then coroutines on every core

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

Each task is an operating-system thread, and tasks take turns ([decision 0020](0020-tasks-take-turns.md)). With two tasks, a turn passes in well under a microsecond: the next thread spins for it ([benchmarks/net](../../benchmarks/net/README.md)). With many, it doesn't. A thread waits through many turns before its own, falls asleep, and the system takes tens of microseconds to wake it on another core.

An echo server with 100 clients making 1,000 round trips each takes Kavach 5.5 seconds. C++ with a thread per connection takes 93 ms on 24 cores, and 345 ms on one core, where its threads switch cheaply. A server is exactly a program with many tasks.

## Decision

**Tasks become stackful coroutines, and later run on every core, as Go's goroutines do.** Mithlesh chose this on 2026-11-03, with the `corosensei` crate for switching stacks.

It comes in three steps, each landed and measured on its own:

1. **Coroutines on one thread (native code).** A task is a coroutine with its own stack, run by the thread of the main task. Passing the turn switches stacks, with no system call. What a thread kept for its task (which task it is, its stack limit, whether it is cancelled, the tasks it spawned, the last error, its agent and file scopes) becomes the task's context, saved and restored at each switch. The turn order is the scheduler's, as now, so programs print the same. Tasks no longer run ahead of their turn on threads of their own ([decision 0035](0035-library-calls-overlap.md)): mixing such threads with coroutines made a server slower than before (100 clients × 1,000 round trips: 6.0 s with running ahead, 0.91 s without). A task that only computes uses one core until step 3; `parallel_map` and parallel loops still use all cores. Mithlesh chose this on 2026-11-03. Tasks spawned off the main thread (from a library call's thread) stay threads; the scheduler passes the turn between the two kinds.
2. **Tasks on all cores** (decision 0020's stage 2). Each task gets its own heap; values cross between tasks as copies (they already do, spec 9.5), and `Shared<T>` is the one value with an atomic count and a lock. The turn order then holds for what tasks print and send, not for when they compute.
3. **Coroutines on every core.** A thread per core runs coroutines, and a task that waits frees its thread for another.

The interpreter keeps a thread per task: it is the reference for what programs do (decision 0014), and its values and recursion can't be split into coroutines with safe Rust (decision 0020). Both use the one scheduler, so they run tasks in the same order.

## Consequences

- Step 1 removes the operating system from each turn. A server with many connections should come close to C++ on one core; steps 2 and 3 bring all cores.
- Each task's stack is reserved address space that the system fills only as it is used, as a thread's is now.
- `corosensei` is a new dependency: x86-64 and ARM64 on Linux, Windows and macOS, with guard pages and panics that unwind safely. Its switching code is `unsafe`, inside the crate; our use of it lives in `runtime/native`, where `unsafe` is allowed ([decision 0016](0016-native-runtime-and-executables.md)).
- A task that blocks inside a system call (a file read) holds its thread, as it holds the turn today. File I/O through the readiness queue or io_uring stays in the to-do.

## Alternatives considered

- **Keep threads, tune the hand-over:** waking only the next thread and letting it spin brought two tasks below C++ (110 ms against 310), but many tasks wake sleeping threads all the same. Waking the next two made it worse (5.5 s → 9.5 s). Keeping every task thread on one core helped on Linux (5.5 s → 1.7 s) and hurt on Windows (6.9 s → 15.8 s).
- **All cores first, without coroutines:** matches C++'s 24 cores, but each turn still costs a thread switch, and a thread per connection doesn't scale to tens of thousands.
- **Our own stack-switching assembly:** no dependency, but per-platform code to own and audit.
