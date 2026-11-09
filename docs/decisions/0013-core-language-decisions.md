# 0013. Core language decisions for Phase 1

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Writing conformance tests for chapters 1–7 exposed open questions that the Phase 1 compiler must answer. Each is small. They are recorded together here so the reasoning is in one place.

## Decision

| Question | Decision | Spec |
|---|---|---|
| `Int / Int` | Stays an `Int` (truncated). No hidden conversions | 1.8 |
| Names mixing scripts | Warning, against look-alike attacks | 1.5 |
| How values print | Like literals: `7.0`, `499.00`, `[1, 2]`, `User(name: "Asha")`; `Display` to customize | 1.11 |
| Multi-line strings | Line breaks next to the `"""` delimiters are not part of the string | 1.7 |
| `BigInt` | Not built in; `std.math.BigInt` | 2.1 |
| Map and set order | Insertion order | 2.2 |
| Structural interfaces | No, not even for interop types; use `extend T: I` | 2.8 |
| Ignoring a call's result | Compile error; discard on purpose with `let _ =` | 3.7 |
| Tail calls | Not guaranteed; stack overflow is a panic | 3.8 |
| Pipe operator `\|>` | No; method chains and `extend` cover it | – |
| Labeled `break` | No; move nested loops into a function and `return` | 4.6 |
| Error context | `try call() context "text"` | 5.7 |
| Exit behaviour | `main` error: stderr, exit 1. Panic: stderr, exit 101 | 5.8 |
| Error traces | In debug builds, errors record each `try` they pass | 5.8 |
| Cycle collector | No; debug builds detect leaked cycles | 6.2 |
| Arena escapes | The block's value is the only way out; storing outside is an error | 6.3 |
| Capabilities in `drop` | Allowed; required where the value is created | 6.1 |
| Quantifiers in contracts | No new syntax; `.all(…)` and `.any(…)` | 7.5 |

## Consequences

- Phase 1 can start with no open questions in chapters 1–7 except loop invariants (Phase 3).
- Output and tests are deterministic: insertion-ordered maps, literal-style printing.
- The language stays small: no pipe operator, no labels, no structural interfaces, no quantifier syntax. Each can be added later without breaking code; removing one later would break code.

## Alternatives considered

- **Python-style `Int / Int -> Float`:** familiar, but hides a conversion and loses exactness.
- **Unspecified map order:** a little faster, but makes output and tests flaky.
- **Allow ignoring results silently:** convenient, but hides real bugs, especially with value types where every update returns a copy.
- **Guaranteed tail calls:** not portable across the Cranelift, LLVM and WASM backends today.
- **A cycle collector in debug builds:** would hide leaks that release builds still have.
