# 0097. `take()` gives `Taken<T>`

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

`receive.take()` gave the next value as `T?`, and `none` once the channel was closed and empty. For a `Channel<T?>` that is `T??`, and optionals don't nest in Kavach (2.7): a `none` put into the channel looked exactly like a closed channel. A pool or a feed that sends `none` for "nothing this time" was cut off at its first one. The PostgreSQL package found this (2026-11-12). A maintainer chose (2026-11-12), from the options put to them, the type below.

## Decision

- `receive.take()` gives `Taken<T> = Item(value: T) | Closed`, from `std.tasks`, which every program that uses a channel gets.
- `for v in receive` stops only at `Closed`, so a channel of optionals carries `none` too.
- `.timeout(d)` on `take()` gives the same `Taken<T>`, and fails when the time runs out.

## Consequences

- `receive.take() ?? x` no longer fits: a program matches `Item` and `Closed`, or uses `for`.
- Native code takes a value and a flag for the end, with no optional made for each value.

## Alternatives considered

- **Refuse optional item types:** simpler, but a feed of optional values must be wrapped by hand.
- **Keep `T?` and document it:** the trap stays.
