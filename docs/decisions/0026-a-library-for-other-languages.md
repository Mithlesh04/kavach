# 0026. A library for other languages through the C ABI

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

Programs already in use (Node.js, Java, Go, Python, C#) should be able to call Kavach code. The platform plan ([platform.md](../vision/platform.md#tcp-udp-and-http)) asks for two ways: over the network (HTTP, gRPC, Kavach Link) between processes, and inside one process, with a library those languages can load. Every one of them can call C: Node-API and `ffi`, JNI and Java's FFM API, cgo, P/Invoke, `ctypes`. Kavach already calls C (spec 10.7), so its native code knows the C types and calling convention.

## Decision

**`kavach build --lib` builds a shared library with a C ABI, and a C header for it.**

1. A library package (`src/lib.kav`) builds into `.kavach/lib/`: `<name>.dll` on Windows, `lib<name>.so` on Linux, `lib<name>.dylib` on macOS, with `<name>.h`.
2. Every `pub fn` of `src/lib.kav` with only C types (numbers, `Bool`, `Ptr`) that can't fail is exported as `<package>_<fn>`. The prefix keeps the names apart from other libraries'. Other `pub` functions are listed, with why they aren't exported.
3. Each export is a small C function in front of the Kavach one: it sets up the runtime on the first call (the program's types, its path for panic messages) and the stack limit of the calling thread, then calls the Kavach function, which already follows the C calling convention.
4. The library carries its own copy of the runtime, hidden: on Linux its symbols are made local (`--exclude-libs`), and a Windows DLL exports only what it lists. A Kavach program loading the library keeps its own runtime.
5. Native code only, built with Cranelift for now.

## Consequences

- Any language that can call C can call Kavach, in the same process, with no network.
- A panic in the library ends the whole process, as it does in a Kavach program. Callers must know this.
- Calls must not overlap: the runtime's heap is not shared between threads yet. A caller with threads keeps calls to one at a time.
- Text, records by value and errors don't cross yet: what crosses is what C passes as it is. Text both ways and errors as return codes come next.
- Two Kavach libraries in one process each have their own runtime and heap; values can't move between them except as C data.

## Alternatives considered

- **A library per language** (a Node-API addon, a JNI library, …): nicer to call, but one build per language and per runtime version. The C ABI is the one thing they all share; wrappers per language can be generated on top of the header later.
- **Exporting every function under its Kavach name:** names would clash with other libraries and with C's own (`free`, `open`).
- **One shared runtime library (`libkavach.so`) for all Kavach libraries:** smaller, but every library would then need the same runtime version, and installing one more file is one more thing to get wrong.
