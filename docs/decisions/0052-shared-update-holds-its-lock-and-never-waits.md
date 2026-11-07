# 0052. `Shared.update` holds its lock, and never waits

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

`counter.update(n => n + 1)` must be safe from any task (spec 6.2). On one core it is: tasks take turns, so no other task runs between reading the value and writing the new one. On all cores ([decision 0049](0049-tasks-on-all-cores-keep-the-programs-order.md)) two tasks can read the same value at once, and one update is lost.

Mithlesh asked for the best call for speed and simple, clear code (2026-11-03).

## Decision

**`update` holds the value's lock while its function runs, and that function may not wait.**

- Each `Shared` value has a lock. `get`, `set` and `update` take it; `update` keeps it until its function returns, so updates from many tasks never lose one.
- The function given to `update` may not wait: no channel `put` or `take`, no `.wait()` on a task, no `sleep`, no network or file call, no `ask`. Where the checker sees the wait in the function itself, it is a compile error; a wait reached through another function panics.
- Values going in and out are copies when tasks run on several cores, as for channels.

## Consequences

- An update is a few instructions under a lock held for nanoseconds: fast on every core, and the lock can never be held across a wait, so a cancelled task can't leave it locked.
- A `Shared` value can't deadlock through `update`: nothing inside it waits for another task.
- Code that needs a slow value for its update gets it first: `const page = try fetch(url)`, then `cache.update(c => c.with(url, page))`.

## Alternatives considered

- **A lock that may be held across waits** (as Go's `sync.Mutex`): an update could wait for the network while every other task wanting the value waits too, and a task cancelled during that wait would need the lock released for it.
- **Retry when another task got there first** (compare and swap): never blocks, but the function may run several times, so it must have no effects, and that can't be checked.
- **`Shared` on one core only:** simple, but programs that share a counter couldn't use all cores.
