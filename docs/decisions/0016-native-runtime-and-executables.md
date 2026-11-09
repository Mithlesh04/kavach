# 0016. A native runtime library, linked into executables

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Native code ran only in memory (`kavach run --native`): Cranelift compiled it inside the `kavach` process and called runtime functions by their address there. Programs ship as executable files ([decision 0005](0005-backends-cranelift-llvm-wasm.md)), so `kavach build` must write one. An executable can't call into the compiler process: the runtime (heap values, printing, panics) must be inside the executable, and the generated code must not contain addresses from the compiler.

The runtime is also `unsafe` Rust (turning object pointers back into objects). [AGENTS.md](../../AGENTS.md) allowed `unsafe` only in `compiler/backend/cranelift`.

## Decision

- The runtime moves to its own crate, `runtime/native` (`kavach-runtime-native`). It is built two ways: as a Rust library that the compiler uses for in-memory runs, and as a static library (`kavach_runtime_native.lib` or `libkavach_runtime_native.a`) that `kavach build` links into executables.
- Every function native code calls is exported by name (`#[unsafe(no_mangle)] extern "C"`). In memory, the JIT finds them by address; in an executable, the linker finds them by name.
- Generated code holds no compiler addresses: text constants are data in the module.
- `kavach build file.kav` writes an object file with Cranelift (`cranelift-object`), adds a C `main` that hands the entry function, the file path and the table of heap types to the runtime's `kv_main`, and links it with the platform's linker: MSVC `link.exe` on Windows (found like Rust finds it), `cc` on Linux and macOS.
- `unsafe` Rust is allowed in `runtime/native` too, with the same rule as the backend: each use has a `SAFETY:` comment.
- The conformance cross-check builds every supported program and runs the executable as well, so both ways must match the interpreter.

## Consequences

- `kavach build` makes small, fast executables (about 0.5 MB, linked in a fraction of a second) that need nothing else to run.
- Building needs a linker on the machine: the C++ Build Tools on Windows, a C compiler elsewhere. Rust needs the same, so Kavach developers who build the compiler have it already. Bundling a linker (such as `lld`) can remove this later.
- A release of Kavach ships the runtime library next to the `kavach` executable.
- Linking works on Windows and Linux (x86-64), where the whole test suite passes, including building every supported program as an executable. macOS is written but not yet tested.

## Alternatives considered

- **Executables that carry the compiler and compile at start-up:** no linker needed, but large files and a slow start, and not real native code.
- **Writing our own linker:** too much work for now; using the system linker is what Rust, Go (with cgo) and Zig (optionally) do.
- **Waiting for the LLVM backend:** it needs the same runtime library and linking, so nothing is gained by waiting.
