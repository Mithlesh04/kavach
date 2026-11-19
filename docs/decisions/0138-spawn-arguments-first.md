# 0138. A spawned call's arguments are worked out at the spawn

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

Spec 9.1 ran a spawned expression in its new task, all of it. So in `spawn cores total(xs.slice(0, 1000))` the task got all of `xs` and cut it there: each task copied the whole list. Sakshi measured it (to-do item 10, S16): 1M records split into 8 tasks cost 2.3 s for flat records and up to 9.5 s for nested ones, against 0.1 s to make the same records inside the tasks. kavach-sdc's `check` stayed on one core because of it.

How other languages do it:

- **Go:** `go f(x)` works out `f` and `x` in the calling goroutine, then the new goroutine runs the call. A panic in `x` happens at the `go` line.
- **Rust:** `thread::spawn(move || f(x))` takes a closure; what it captures is moved in at the spawn, and the call runs in the thread.
- **JavaScript:** `new Worker(...)` with `postMessage(x)`: the message is copied when it is sent.

## Decision

A maintainer chose (2026-11-19) Go's rule. In `spawn f(a, b)` and `spawn x.m(a, b)`, the spawning code works out the value before the dot and the arguments, in the order they are written; the task then makes the call with those values. A panic there happens at the spawn's line, and a `try` there passes its error on from the spawning function.

The call itself, a spawned block, a call through `?.`, `w.timeout(d)`, `inout` arguments and the list a changing method is called on still run in the task (spec 9.1 has the list). To work an argument out in the task, spawn a block: `spawn { total(xs.slice(0, 1000)) }`.

The checker finds the parts once (`compiler/checker/src/spawn_parts.rs`); the interpreter and code generation both read that list, so they always agree.

## Consequences

- A task copies only what its call is given. Splitting a list across cores costs what cutting it costs.
- A panic in an argument points at the spawn, where the mistake is, not at a task.
- A program that relied on an argument being worked out in the task behaves differently. None did in the conformance tests or examples, and Kavach has no users yet.

## Alternatives considered

- **Keep running the whole expression in the task, and make the copy faster.** The copy was made 35–45% faster (2026-11-19), but each task still copied the whole list: 8 tasks, 8 copies.
- **Have the compiler find the slice and cut it before the task, as an optimisation.** `lower_tasks/args.rs` did this for field paths. It can't be done in general without changing when a panic happens, and a rule a reader can't see is harder to trust than one they can.
