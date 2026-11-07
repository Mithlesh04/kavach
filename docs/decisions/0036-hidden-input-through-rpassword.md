# 0036. Passwords are read unseen through `rpassword`

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

Programs ask people for passwords and tokens (Mithlesh, 2026-11-02). Typing them must not show on the screen, and the answer must not end up in a log. The second part Kavach already has: a `secret` field (spec 2.11, 8.7) is kept out of `print`, logs, AI prompts and error messages by the compiler.

Not showing the typing needs the terminal's echo turned off while the line is read: `tcsetattr` on Linux and macOS, `SetConsoleMode` on Windows. Both are system calls through C, so `unsafe` Rust. `unsafe` is allowed only in `runtime/native` (decision 0016), and the interpreter (`kavach run`), which doesn't use that crate, must read passwords too.

## Decision

**`runtime/core` reads a hidden line with the `rpassword` crate (version 7), and `std.term` builds `term.ask_secret` on it.**

1. `io.read_hidden() -> String?` (spec 14.6) reads a line without showing it when standard input is a terminal, and an ordinary line otherwise (a pipe, a test), so scripts can still answer.
2. `term.ask_secret(question) -> Secret?` (spec 14.13) gives `Secret(text: …)`, whose `text` is a `secret` field: the compiler keeps it from being printed or logged, and `reveal` is needed to use it as plain text.
3. `rpassword` brings `rtoolbox` (by the same author) and `windows-sys` 0.61; `libc` was already a dependency.

## Consequences

- Passwords are typed unseen in the interpreter and in native code alike, with no `unsafe` of ours.
- Three more crates to keep up to date. They are small, widely used (`rpassword` is the usual choice in Rust tools), and do only this.
- `io.read_hidden` itself gives a plain `String`; only `term.ask_secret` makes it a secret. A program that reads a password with `io.read_hidden` directly must keep it secret itself.

## Alternatives considered

- **Our own `unsafe` calls in `runtime/core`:** no new crates, but `unsafe` outside `runtime/native` (against decision 0016) for code that `rpassword` already gets right on every platform.
- **`stty -echo` through a child process:** no `unsafe`, but Unix only; Windows has no `stty`.
- **A bigger terminal crate (`crossterm`):** does much more than this needs, with many more dependencies.
