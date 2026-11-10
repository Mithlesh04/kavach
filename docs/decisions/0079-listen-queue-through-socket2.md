# 0079. Listeners wait in the system's longest queue, through `socket2`

- **Status:** Accepted
- **Date:** 2026-11-10 (Mithlesh chose to add `socket2`)

## Context

A listening socket keeps the connections that arrived but weren't accepted
yet in a queue, its backlog. `net.listen` (spec 14.10) made its socket with
Rust's `TcpListener::bind`, which always asks for 128 places, and so does
`mio`. When more connections arrive than that before the program accepts
them, the system drops the rest: on Linux a dropped client tries again
after a second, then three; Windows refuses it.

Measured on WSL, a server that accepts in a loop while 500 clients connect
at once, each making one round trip: 3.3 s, sometimes 1.2 s, where the
work takes a fraction of a second. `redis` asks for 511, nginx 511, Go for
the system's limit (`/proc/sys/net/core/somaxconn`, 4,096 here).

Setting the queue's length means calling `listen` with it, which Rust's
standard library doesn't offer. Calling it directly needs `unsafe`, which
`runtime/net` may not use (AGENTS.md, section 8).

## Decision

`runtime/net` depends on `socket2`, the crate tokio and many others use for
socket options. `net.listen` and `net.listen_tls` make their socket with
it and ask for the longest queue the system allows (`i32::MAX`, which Linux
and macOS cut to their limit, and which Windows reads as `SOMAXCONN`, its
own reasonable maximum).

## Consequences

- Many clients connecting at once are queued, not dropped: none waits a
  second to try again until the system's own limit is reached.
- One more dependency, small and widely used, with no dependencies on
  Windows beyond `windows-sys` and on Unix beyond `libc`.
- The queue's length isn't a choice a Kavach program makes; if one ever
  needs to, `net.listen` can take it as an option (spec 14.10).

## Alternatives considered

- **`unsafe` calls to `listen` in `runtime/net`:** a second place that
  may use `unsafe`, for what a common crate already does safely.
- **Keep 128:** servers that get bursts of connections stall for seconds.
- **Accept faster:** a program accepting in a loop already accepts as fast
  as its turn comes; a burst arrives faster than any loop.
