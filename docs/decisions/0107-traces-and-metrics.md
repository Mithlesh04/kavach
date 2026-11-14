# 0107. Traces and metrics in `std.log`; blocks given to a call

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

`std.log` sends events to an OpenTelemetry collector (decision 0023, spec 14.12). Rath, a web framework written in Kavach, needs the other two parts of observability: traces, which follow one request across tasks and services, and metrics, which count and measure. They belong in `std.log`, not in a package: a package can't start a sender that outlives its functions (9.2), and log records must carry the trace themselves.

A trace needs a scope (the code that runs as one span) and a way to reach code and tasks under a request without passing it everywhere. A seeded random generator (language request 9 (9)) needs the same kind of scope. A maintainer chose every option below (2026-11-14).

## Decision

- **Blocks given to a call** (spec 4.8): `log.span("name") { … }` and `random.seeded(seed) { … }`. The block is a plain block, not a lambda: `try`, `return` and `break` work as without the call, and it can change the variables around it. What the call set up ends however the block ends. Only standard library functions that say so take a block.
- **Spans as blocks** (spec 14.12). A span in a span is its child; a failing block marks its span as an error.
- **The trace is kept per task** and handed to the tasks it spawns, as the folders of `uses Fs("…")` are, so no signature changes.
- **`http` joins traces by itself:** `http.serve` runs each handler in a span that continues the request's W3C `traceparent`, and `http.request` sends the current one.
- **Metrics as calls:** `log.count`, `log.gauge` and `log.histogram`, kept by the runtime and sent as OTLP metrics every 10 seconds. Like logging, they need no capability.
- **Seeded numbers** come from xoshiro256**, seeded through SplitMix64, named in the spec so they never change.

## Consequences

- Traces join across Kavach services, and with other languages' services through the W3C header, with no code in handlers.
- Every log record inside a span carries its trace and span ids, so a collector links events to the request.
- The runtime's per-task scope stack (folders, programs) also holds the current span.
- Seeded runs repeat exactly, for games and simulations; tasks spawned in a seeded block draw from the system.
- A program's own functions can't take a block yet: an open question.

## Alternatives considered

- **Explicit `Span` values** (`start`, `end`): more control, but a span left unended is a common bug.
- **Passing the trace by hand:** explicit, but every function on a request's path would need a `Trace` parameter.
- **Only helpers for `traceparent`:** each program would wire them itself, and most would forget.
- **Typed metric records:** checked names, but more to write than the call forms other languages use.
- **Lambdas instead of blocks** (`log.span("name", () => { … })`): no new syntax, but a lambda's `try` and `return` are its own (3.3), and it can't change the variables around it.
