# 0122. Floats give the same bits everywhere

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

A game played online by two players, a replay, a simulation checked by a test: each needs the same numbers on every machine (to-do 9 (9)). IEEE 754 makes `+ - * /` and `sqrt` exact, so they agree everywhere, unless a compiler fuses a multiply and an add into one step (FMA) or reorders a sum. But `sin`, `exp`, `pow` and the like aren't exact: each system's C library works them out its own way, so `sin(1e22)` can differ in its last bit between Windows and Linux. Kavach called the system's functions, through Rust in the interpreter and through LLVM's intrinsics in built programs; LLVM could also work out a constant call with the compiling machine's library. A maintainer chose on 2026-11-16, from options with a recommendation: always the same bits, with no mode to turn on.

## Decision

- **Every `Float` and `Float32` result is the same everywhere**, on the CPU (spec 2.1): every machine, system and build, interpreted or native.
- **No fused multiply-add, no reordered sums.** Kavach already never asks for either (decision 0031 keeps a sum's order); this makes it a rule. `a.product(b, fast: true)` (2.17) stays the one place a program asks for speed over bits.
- **`std.math`'s inexact functions are Kavach's own code:** `pow`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp` and `ln` come from the **`libm`** crate, a pure-Rust port of musl's math library kept by the Rust project (MIT or Apache-2.0), built without its machine-specific paths, so the same code runs on every system. The interpreter, the runtime and LLVM builds all call it; built programs call it through the runtime (`kv_math_*`), never through LLVM's intrinsics, so no machine's library is used, at run time or while compiling.
- **The GPU is apart:** a `gpu fn` runs on the GPU's own hardware, whose functions differ by vendor (chapter 15).

## Consequences

- Lockstep games, replays and expected numbers in tests hold on every machine.
- `libm`'s functions are about as fast as the C libraries'; a loop that calls `sin` a billion times may be a little slower or faster depending on the system.
- `libm` was already in the build through other crates; it becomes a direct dependency of the runtime and the interpreter.

## Alternatives considered

- **A build setting** (`[build] floats = "same"`): the system's functions by default. Rejected: the default would differ between machines, and a program could forget the setting.
- **A `same_bits { }` block:** deterministic code inside it only. Rejected: a new keyword, and easy to leave out of one function.
- **Writing the functions from scratch:** possible, but musl's are tested over decades; porting them again gains nothing.
