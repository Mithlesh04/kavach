# 0095. `Shared.update` gives a result

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

`s.update(f)` changed a `Shared` value under its lock (decision 0052) and gave nothing back. Code that needed to "take something and know what it took" in one step couldn't: a rate limiter taking a token, a pool taking a free connection, an idempotency key being claimed, the next id. Reading with `get()` after the update lets another task's update come between. Rath and the PostgreSQL package asked for it (2026-11-12). A maintainer chose (2026-11-12), from the options put to them, both forms below.

## Decision

- `s.update(f)` gives the new value, read while the lock is still held.
- `s.update_with(f)`: `f` gives a pair, the new value and a result; the call gives the result.
- The value `update` gives may be left unused, unlike other values (spec 3.7), so `counter.update(n => n + 1)` stays a statement.
- Neither function may wait, as before.

```kavach
const id = next_id.update_with(n => (n + 1, n))      // take an id
const total = counter.update(n => n + 1)             // the count now
```

## Consequences

- A token bucket, a pool's free list and claimed keys are one atomic step.
- `update` is the one call whose result may be dropped silently.

## Alternatives considered

- **Only `update` giving the new value:** too little for results that differ from the stored value.
- **Only the pair form:** the common `n => n + 1` would need `(n + 1, ())`.
