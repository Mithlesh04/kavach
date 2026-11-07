# 0049. Tasks on all cores keep the order the program gives

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

[Decision 0048](0048-tasks-as-coroutines-per-core.md) plans tasks on every core. Today one task runs at a time, in a defined order (spec 9.7), so every program prints the same on every run. Once tasks run at the same time, a server's network work on many cores can't all wait for one global order without losing the speed. What should stay in order? The full analysis is in [architecture/tasks-on-all-cores.md](../architecture/tasks-on-all-cores.md).

## Decision

Mithlesh chose (2026-11-03):

1. **What tasks print stays in the order the program gives.** A task's own lines come out in the order it prints them. Between tasks, a line printed before a task sends a value (on a channel, or by ending) comes before what the receiver prints after it, and the same for spawning a task. Lines nothing orders may come out either way, never mixed into one line. The network, files, channels and timers run in parallel. Go, Rust and Java work this way.
2. **All cores by default; the developer can change it.** `tasks` in `kavach.toml` (spec 9.8) gets a third value, `"parallel"`, the new default. `"coroutines"` keeps one core and today's turn order, so every run prints the same: for simulations, and for comparing native code with the interpreter in the test suite. `"threads"` stays as it is.

The spec changes with each step that lands, in the order of the architecture page.

## Consequences

- Servers and pipelines use every core.
- Programs whose output depends on timing alone may print in a different order from run to run under `"parallel"`; `"coroutines"` is there for those that need the same output every time.
- Every runtime structure that assumes one thread at a time has to change (the architecture page lists them), in about three to four weeks of work.

## Alternatives considered

- **The interpreter's order in every program (deterministic multithreading):** every run prints the same, but every print waits on a logical clock of all tasks. Slow, and much more work.
- **One core by default, all cores when asked:** repeatable by default, but most programs would leave the speed unused.
