# 0005. Cranelift for development, LLVM for release and WASM

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Developers need instant feedback (`kavach run` should feel like running a script). Production needs maximum speed on many platforms, including the web.

## Decision

- **Cranelift** for development builds: compiles very fast with good-enough speed.
- **LLVM** for release builds: maximum optimization, and every major CPU plus WASM.
- Both consume the same Kavach IR (KIR), so behavior is identical.

## Consequences

- Fast edit-run loop and fast release binaries.
- Two backends to maintain. KIR must stay simple enough to lower cleanly to both.
- Phase 1 ships Cranelift only; LLVM arrives in Phase 3.

## Alternatives considered

- **LLVM only:** slow debug compiles hurt the "easy" goal.
- **Interpreter for development:** instant start, but debug and release behavior could drift apart.
- **Compile to C:** portable, but weaker debugging and less control.
