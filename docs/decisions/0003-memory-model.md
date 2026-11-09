# 0003. Automatic memory without GC, with opt-in control

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Goals: easy like a garbage-collected language, faster than C#, no GC pauses, and freedom for developers who need full control.

## Decision

Four levels (spec chapter 6):

1. **Automatic (default):** value semantics. Ownership is inferred, reference counting is optimized away at compile time where possible (Perceus-style), and values are reused in place when uniquely owned.
2. **Shared state:** explicit `Shared<T>` and `Weak<T>`.
3. **Arenas:** `arena { }` for bulk allocation and freeing.
4. **Unsafe:** raw pointers, opt-in per package and listed by `kavach audit`.

## Consequences

- No GC pauses, predictable latency, and resources freed at a known point.
- No borrow checker for developers to fight; the compiler does the ownership work.
- Cycles are possible only through `Shared`, and debug builds detect leaked cycles.
- The compiler work (ownership inference, removing reference-count updates) is significant and is a core research risk for Phase 1.

## Alternatives considered

- **Tracing GC (C#, Go, Java):** easiest to implement, but pauses and memory overhead conflict with "faster than C#".
- **Rust-style borrow checking:** the fastest and safest, but the learning curve conflicts with "productive in one day".
- **Manual (C, Zig):** maximum control, but unsafe by default.

Prior art: Swift (ARC), Koka (Perceus), Lean 4, Nim (ORC), Vale.
