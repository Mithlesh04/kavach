# 0074. `gimli` writes the unwind tables of `kavach build`

- **Status:** Accepted (2026-11-05, Mithlesh agreed)
- **Date:** 2026-11-05
- **Changes:** `compiler/backend/cranelift` (a direct dependency)

## Context

Decision [0065](0065-a-panic-stops-only-its-task.md) makes a panic stop only its task, by unwinding, in three steps; the first is the object files `kavach build` writes with Cranelift. 0065 assumed those files already carry unwind information. They don't: Cranelift describes each function's frame (`create_unwind_info`), but `cranelift-object` writes no unwind section. Linux wants `.eh_frame`, DWARF call-frame information; Windows wants `.pdata` and `.xdata`.

`.pdata` and `.xdata` are simple tables, written by hand. `.eh_frame` is DWARF, with its own encodings of addresses and instructions, and Cranelift hands each function's part over as `gimli` types (`to_fde`).

## Decision

The Cranelift backend uses `gimli` (0.32, the `write` feature) directly to write `.eh_frame`, as `rustc_codegen_cranelift` does. It is already in the build: Cranelift depends on the same version. A writer of our own notes where each function's address goes, so the linker fills it in, written relative to its place (PC-relative), so a position-independent executable needs no fix-ups.

## Consequences

- No new crate enters the build, only a direct use of one that is there; it moves with Cranelift's version.
- `kavach build` with Cranelift writes unwind tables for every compiled function (`compiler/backend/cranelift/src/unwind.rs`). Nothing reads them yet: a panic still ends the program until the runtime unwinds (0065's later steps).

## Alternatives considered

- **Writing `.eh_frame` by hand:** no dependency, but about 200 more lines of DWARF encoding to keep correct.
- **Holding 0065:** panics stay fatal, and every task's bug takes the whole program down.
