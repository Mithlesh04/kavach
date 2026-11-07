# 0018. The LLVM backend writes LLVM IR text and uses rustup's LLVM tools

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

[Decision 0005](0005-backends-cranelift-llvm-wasm.md) chose Cranelift for development and LLVM for release builds, with LLVM planned for Phase 3. The speed work ([to-do item 1](../vision/todo.md)) has reached what Cranelift can do: on the `matrix`, `fib` and `floats` benchmarks, the remaining gap to C++ is register allocation, loop optimization and vectorization, which LLVM has and Cranelift doesn't. So LLVM moves forward.

The usual way to use LLVM from Rust is to link its C++ library (`inkwell`, `llvm-sys`). That needs an LLVM development install of the exact version on every machine that builds the compiler, which is hard on Windows and adds minutes to every build.

## Decision

- A new crate, `compiler/backend/llvm` (`kavach-backend-llvm`), turns KIR into LLVM IR **as text** (a `.ll` file). It runs `opt -O3` and `llc` on it to get an object file, then links it with the runtime library exactly like the Cranelift backend (decision 0016).
- `opt` and `llc` come from rustup's `llvm-tools` component (`rustup component add llvm-tools`), the same LLVM version that Rust itself uses. `KAVACH_LLVM_BIN` can point at another LLVM's `bin` folder.
- Every local is a stack slot (`alloca`); LLVM's own passes turn them into registers. The generated code calls the same runtime functions and reads the same object layout as the Cranelift backend, so behaviour is identical.
- `kavach build` uses the LLVM backend when the tools are found and the program uses only what the LLVM backend supports so far; otherwise it uses Cranelift, as before. `KAVACH_BACKEND=cranelift` forces Cranelift. `kavach run --native` keeps using Cranelift (fast to compile).
- The conformance cross-check builds every program with `kavach build`, so whatever the LLVM backend supports is checked against the interpreter.

## Consequences

- Release executables get LLVM's optimizer with no LLVM library in the compiler and no change to how Kavach is built.
- Building with LLVM needs the `llvm-tools` component; without it, Kavach falls back to Cranelift and still works.
- Two backends to keep in step. Both read KIR, and the cross-check runs both, so a difference shows up as a failing test.
- The LLVM backend grows feature by feature; until it supports everything, some programs are still built with Cranelift.

## Alternatives considered

- **Linking LLVM as a library (`inkwell`):** faster to compile (no text, no separate processes), but needs an LLVM development install on every machine.
- **Compiling to C and using `gcc` or `clang`:** rejected in decision 0005 (weaker debugging and control), and it would still need a C compiler with a good optimizer on Windows.
- **Staying with Cranelift:** it compiles fast but doesn't vectorize or optimize loops as LLVM does; the remaining benchmark gaps would stay.
