# 0007. Errors are values; no exceptions

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Exceptions hide failure paths: any call might throw, and the signature doesn't say so. In code that moves money or runs agents, unhandled failure paths are the most common source of bugs.

## Decision

A function that can fail declares `! ErrorType`. Callers must `try`, `catch` or `match`. `fail` returns an error. Bugs (contract violations, out-of-range indexes, overflow) are panics, which normal code does not catch (spec chapter 5).

## Consequences

- Every failure path is visible in signatures and impossible to ignore.
- `try` keeps the "happy path" short.
- `! Error` gives scripts an easy catch-all.

## Alternatives considered

- **Exceptions (C#, Java, Python):** familiar, but invisible failure paths.
- **`Result<T, E>` with `?` (Rust):** the same idea, but `T ! E` with `try` reads more plainly, and `?` is already used for optionals.
