# 0063. Ctrl+C and `SIGTERM` through `libc` and `windows-sys`

- **Status:** Accepted (2026-11-04)
- **Date:** 2026-11-04
- **Changes:** `runtime/native` gains two direct dependencies

## Context

[Decision 0062](0062-crashes-restarts-and-clocks.md) says Ctrl+C, `SIGTERM` and closing the console end a program as `cli.exit` does: the end hooks run, output and logs are flushed, and the exit code is 130 or 143 (spec 5.8). A second Ctrl+C ends it at once. Kavach catches no signal today, so the system kills the program.

Catching them takes the operating system's own calls. On Unix that is `sigaction` or `sigwait`; on Windows it is `SetConsoleCtrlHandler`. Both are `unsafe`, which only `runtime/native` may use ([decision 0016](0016-native-runtime-and-executables.md)).

## Decision

`runtime/native` catches the signals itself, through `libc` on Unix and `windows-sys` on Windows. Both crates are already in the dependency tree (other crates use them); they become direct dependencies of `runtime/native` only.

- **Unix:** `SIGINT`, `SIGTERM` and `SIGHUP` are blocked when the program starts, before any other thread, and one thread waits for them with `sigwait`. When one arrives, that thread runs the end hooks in ordinary code, not inside a signal handler, where almost nothing is allowed.
- **Windows:** `SetConsoleCtrlHandler` calls its handler on a thread of its own. Ctrl+C and Ctrl+Break count as `SIGINT` (130), closing the console as `SIGHUP` (129), and logging off and shutting down as `SIGTERM` (143). Windows ends the program when the handler returns from a close, so the end hooks run inside it.
- The `kavach` command installs the same handler for `kavach run`, with the interpreter's end hooks.

## Alternatives considered

- **The `ctrlc` crate.** It is widely used, but its handler isn't told which signal came, so 130 and 143 can't be told apart. It also adds `nix` to the tree.
- **`signal-hook`.** It works on Unix only, so Windows would still need its own code.
