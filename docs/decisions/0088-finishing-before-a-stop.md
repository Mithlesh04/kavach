# 0088. Finishing before a stop: holds, and servers that drain

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

A stop signal (Ctrl+C, `SIGTERM`, a closed terminal) ended a program at once, its output flushed ([decision 0063](0063-stop-signals-through-libc-and-windows-sys.md)). Docker, Kubernetes and systemd stop a service with `SIGTERM` and wait a few seconds before killing it, so a server that ends at once cuts off the requests it was answering. Rath, a web framework written in Kavach, asked (2026-11-12) for a graceful drain.

A maintainer chose (2026-11-12) both of the options put to them: servers that drain by themselves, and a general hook any program can use.

## Decision

- **`cli.on_stop(grace) -> StopHold`** (spec 5.8, 14.6): while a hold is kept, the first stop signal doesn't end the program. `hold.asked()` turns `true`, and the program ends, with the signal's exit code, once every hold is let go of (`hold.done()`), when the longest `grace` has passed, or when `main` returns. A second signal ends it at once, as before.
- **`http.serve` drains by itself** (spec 14.11): it takes a hold for `ServeLimits.drain_timeout` (10 seconds), and on a stop it answers new connections with 503, closes those waiting for their next request, finishes the requests under way, then lets go. A plain server needs no code.
- A hold needs no capability: it changes only when the program ends, and only for at most its `grace`.

## Consequences

- A container stopped by `SIGTERM` finishes its responses inside the usual 10-second window, with no code in the program.
- Programs that write files or empty queues get the same chance through `cli.on_stop`.
- A program can't refuse to stop: every hold has a `grace`, and a second signal ends it at once.

## Alternatives considered

- **Only servers drain:** simpler, but a program with other work to finish would have no way to.
- **Only the general hook:** every server would have to write the same drain code.
- **A channel the signal is sent on** (`Receiver<Unit>`): it needs a task waiting on it, and a program without one would still end at once.
