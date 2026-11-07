# 0004. Write the first compiler in Rust

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

The first compiler must be fast, memory-safe and buildable by a small team, and it must have good libraries for parsing, Cranelift and LLVM.

## Decision

Write the compiler, runtime and tools in Rust. Self-hosting (rewriting the compiler in Kavach) is considered only after v1.0.

## Consequences

- Direct access to Cranelift (a Rust library) and mature LLVM bindings.
- Memory-safe compiler and runtime, which matters for a security-focused language.
- Contributors need Rust. This is acceptable: most compiler contributors already know it or want to.

## Alternatives considered

- **C++:** the most direct LLVM access, but slower to develop safely.
- **Go:** fast to write, but weaker for low-level runtime work.
- **OCaml / Haskell:** excellent for compilers, but small contributor pools.
- **Self-host from day one:** impossible before the language exists.
