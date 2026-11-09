# 0046. The I/O event loop, through `mio`

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

A task that waits for a socket (spec 14.10) handed its turn on and tried again after a sleep of 1 ms, doubling up to 8 ms. That is simple and always correct, but each wait costs at least a sleep. On Windows a 1 ms sleep lasts a whole timer tick, 15.6 ms. Two tasks passing a message back and forth 10,000 times over TCP took 156 seconds on Windows and 11.9 seconds on Linux; C++ with blocking sockets takes 0.3 seconds ([benchmarks/net](../../benchmarks/net/README.md)).

The system can say when a socket is ready: epoll on Linux, kqueue on macOS, IOCP on Windows. The Rust standard library doesn't expose them, and `unsafe` is allowed only in `runtime/native` ([decision 0016](0016-native-runtime-and-executables.md)), while the interpreter shares the network code. So it takes a crate.

## Decision

**A waiting task sleeps until the system says its socket is ready, through `mio`.** Mithlesh approved the dependency (2026-11-03).

1. Plain TCP listeners and connections are registered with one readiness queue (`runtime/net/src/net/poller.rs`). Each counts its events. A task notes the count before it tries an operation, and waits only if no event came since, so an event between the try and the wait is never lost.
2. The queue has no thread of its own. When no task is ready but some wait for a socket, the scheduler's thread waits on the queue itself, up to the next timer (`tasks::set_io_idle`); a program without tasks waits on it alone. Its events ready the waiting tasks directly, so a wake-up costs one switch between threads, as a blocking read does in C.
3. The scheduler counts tasks waiting for a socket: while one does, nothing running is not a deadlock. A wake-up that comes just before its task starts waiting is kept, and that wait ends at once.
4. The interpreter and native code wait the same way, through the one scheduler (decision 0020).
5. TLS, UDP, QUIC and connecting still try again after a short sleep. They move to the queue one by one.

## Consequences

- 10,000 round trips: 156 s → 0.6 s on Windows, 11.9 s → 0.43 s on Linux (C++: 0.3 s, Python: 0.45 s).
- `mio` adds one crate: its own dependencies, `libc` and `windows-sys`, were already in the build.
- The network's code sets the scheduler's hook, so a program without networking still doesn't link it ([decision 0024](0024-pay-only-for-what-you-use.md)).

## Alternatives considered

- **`polling`** (smol's poller): the same platforms, but more new crates.
- **Our own epoll, kqueue and IOCP code:** the most code to own, and `unsafe` in `runtime/net`.
- **io_uring:** it pays most for files and for very busy sockets; readiness is how most servers wait for sockets. File I/O through io_uring stays in the to-do.
- **A poller thread of its own:** written first. Each wake-up then took three switches between threads (the poller's, the scheduler's, the task's), and Linux took 1.15 s.
