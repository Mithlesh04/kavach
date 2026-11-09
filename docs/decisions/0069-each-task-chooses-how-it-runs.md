# 0069. Each task chooses how it runs, where it is spawned

- **Status:** Accepted
- **Date:** 2026-11-05

## Context

How tasks run is one setting for the whole program: `tasks = "coroutines"`, `"threads"` or `"parallel"` in `kavach.toml` (spec 9.8, decisions [0048](0048-tasks-as-coroutines-per-core.md) and [0049](0049-tasks-on-all-cores-keep-the-programs-order.md)). But one program often has tasks of all three kinds: a server has a cheap coroutine per connection, one task that blocks in a C library and should have a thread of its own, and some heavy computing that should use every core. With one setting, it can only pick one kind for all of them.

How other languages let the code choose: Kotlin names a dispatcher at each `launch` (`Dispatchers.IO`, `Dispatchers.Default`); Java 21 has `Thread.ofVirtual()` and `Thread.ofPlatform()`; Rust's tokio has `spawn` and `spawn_blocking`; C# has `TaskCreationOptions.LongRunning`. Go chooses for the program, with no say for the code.

## Decision

**Each `spawn` says how its task runs; with no word, it is a coroutine.** Mithlesh chose this on 2026-11-05, and that the `kavach.toml` setting goes.

```kavach
spawn handle(conn)            // a coroutine: the default
spawn thread read_sensor()    // a thread of its own
spawn cores render(frame)     // on any core of the machine
```

- `spawn` makes a coroutine, on the thread of the task that spawned it: cheap, for the many tasks that mostly wait.
- `spawn thread` gives the task an operating-system thread of its own: for a task that blocks inside a C library or computes for long, so it doesn't hold the coroutines' thread. A release build lets it compute ahead until its first effect (spec 9.7).
- `spawn cores` puts the task on a worker thread per core, the next one in turn: for heavy computing. Its effects keep the order decision 0049 gives (a task's own lines in order; a line before a send, a spawn or an end comes before what the receiver prints after it). Builds that can't run tasks on all cores yet (anything but a release build) run it on one core, with the same output.
- `thread` and `cores` are words only right after `spawn`: they stay free as names everywhere else.
- `tasks =` in `kavach.toml` becomes an error that says where the choice went.

Every kind takes turns as spec 9.7 says, so the choice changes how fast a program runs, not what it prints, except that `spawn cores` tasks keep only decision 0049's order.

## Consequences

- One program mixes the three kinds: the choice sits next to the work it is about.
- The runtime runs all three at once: coroutines on the spawner's thread, threads, and per-core workers started when the first `spawn cores` runs.
- Programs with `tasks =` in `kavach.toml` must move the choice into the code. Kavach has no users yet, so nothing outside this repository breaks.
- Decision 0049's plan to make all cores the default for every task is replaced: the default stays coroutines, and code asks for cores where it wants them.

## Alternatives considered

- **The runtime decides (Go):** only `spawn`; the runtime moves a task that computes long, or blocks in C, to another core or thread by itself. Nothing to choose, but no way to force a kind, and a detector to build and tune.
- **Code and a default in `kavach.toml`:** the same per-spawn words, and `tasks =` kept for the default of a plain `spawn`. Two places to look for one choice.
- **A setting per function** (`fn render() runs on cores`): the choice would sit far from the spawn, and one function may be spawned in different ways.
