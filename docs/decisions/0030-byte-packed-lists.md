# 0030. Byte-packed lists: one byte per byte

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

A `[UInt8]` list stores each byte in a 64-bit slot, like every other list. Building 64 MB of bytes one `push` at a time costs about 12 ns per byte — 8× C++ — while the work on the bytes (CRC32C, parsing, encoding) already runs at hardware speed ([benchmarks/bytes](../../benchmarks/bytes/README.md)). Storage engines and binary formats (to-do item 5) live on exactly this: building, slicing and walking byte buffers. `Bytes` itself already stores one byte per byte; the gap is the list used to build it.

## Decision

**Lists of `UInt8` (and the other one-byte types, `Int8` and `Bool`) store their items packed, one byte per byte.**

1. The flat-list machinery already lets a list's slot width differ from one 64-bit slot per item (`runtime/native/src/flat.rs` flattens records into lists). It gains sub-slot widths: a `[UInt8]` has width "1 byte", and `len`, `push`, indexing, `slice` and iteration address bytes, not slots.
2. The representation is invisible: programs read and write `[UInt8]` as before, `Bytes(list)` becomes a copy of the packed buffer (or hands it over when the list is never used again), and `b.to_list()` the reverse.
3. The interpreter keeps its boxed values — it is the semantics reference, not the fast path. The conformance suite proves the packed native lists behave identically.
4. Code generation reads and writes single bytes (`i8` loads and stores); LLVM then vectorizes byte loops it can prove, which is where the rest of C++'s speed comes from.

## Consequences

- `xs.push(UInt8(…))` approaches `push_back` speed, and 8× the memory fits in cache: every binary-format workload moves at once.
- `flat.rs`, both backends' list indexing/push paths, SROA's list handling and `send.rs` copies must learn byte widths: a contained but cross-cutting change, tested by the existing lists and bytes conformance tests plus the `benchmarks/bytes` numbers.
- Mixed-width code (a `[UInt8]` inside a record, lists of lists) keeps working because widths are already per-shape.

## Alternatives considered

- **A separate growable `Bytes` builder type:** fixes the benchmark but leaves `[UInt8]` slow and adds API surface; packing the list fixes the language.
- **Packing every small scalar (16- and 32-bit too):** likely later; one byte first, where the win is 8× and the ABI risk smallest.
