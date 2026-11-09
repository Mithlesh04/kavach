# 0035. Library calls compute at the same time, and take turns for effects

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

`kavach build --lib` makes a C library that other languages load ([decision 0026](0026-a-library-for-other-languages.md)). Until now, calls from several threads ran one at a time behind the library's lock, because the runtime's heap was one. A server in Go or Java that calls a Kavach function from many threads got no more than one core's worth of it.

Since then, tasks in programs built through LLVM compute ahead on their own cores ([decision 0020](0020-tasks-take-turns.md), stage 2 in part). Each thread has its own allocation pool and stack limit, and a task takes its turn only before its first effect another task could see (`turn()` in `runtime/native/src/rt_tasks.rs`). A library call is like such a task: what it gets from C (numbers, pointers, text) is copied into new Kavach values that no other call holds, and Kavach has no global values.

## Decision

**Library calls compute at the same time; each takes the library's lock at its first visible effect, and keeps it until it returns.**

1. A library built through LLVM gives the runtime its per-thread stack limit, as executables do, so pools and stack limits are per thread.
2. A call runs without the lock until its first effect: printing, logging, files, the network, tasks, channels, agents, `ask`, a `Shared` value, a panic. Then it waits for the lock, as a task running ahead waits for its turn, and keeps it until it returns. What different calls print and do never interleaves.
3. A C callback that calls back in from the same call goes on, as before.
4. `<package>_last_error()` gives the last error of the calling thread, like C's `errno`.
5. A library built with Cranelift (when the LLVM tools are missing) keeps the whole-call lock: its generated code takes objects from the one global pool.

## Consequences

- A pure function called from many threads uses as many cores.
- A call that prints early holds the lock for the rest of its run. Printing at the end, or not at all, keeps calls parallel.
- C functions the library calls (spec 10.7) may run in several calls at once. Calling C is `unsafe`, and whether that C code is safe on several threads is the programmer's promise, as it is for tasks running ahead.
- This replaces the consequence "calls must not overlap" of decision 0026; the rest of 0026 stands.
- LLVM-built libraries now check for stack overflow, which they didn't before: the generated code reads the per-thread limit, which nothing set.

## Alternatives considered

- **Keep the whole-call lock:** simple, but a library on a server would use one core.
- **No lock at all, effects included:** prints from two calls would interleave mid-line, and the task scheduler, which a call that spawns tasks uses, is not built for two callers at once.
- **A lock per effect** (taken and given back around each print): more overlap, but the output of one call could be split by another's, unlike a program's tasks.
