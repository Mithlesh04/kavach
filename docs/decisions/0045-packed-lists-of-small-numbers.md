# 0045. Packed lists of 16- and 32-bit numbers

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

[Decision 0030](0030-byte-packed-lists.md) packs lists of one-byte items, one byte per item. Every other list keeps one 64-bit slot per item, so a `[Float32]` uses twice the memory of its numbers.

That costs most where 32-bit numbers matter: the GPU ([decision 0044](0044-gpu-through-wgpu.md)). Its buffers hold 4 bytes per `Float32`. Each `map` on the GPU first copies the list into such a buffer and copies the result back into slots. On a list of 4 million items, the copies and conversions took about 25 ms of the 61 ms a `map` took ([benchmarks/gpu](../../benchmarks/gpu/README.md)). Decision 0030 named this step as likely later.

## Decision

**Lists of 16- and 32-bit numbers store their items packed too: 2 bytes per `Int16` and `UInt16`, 4 bytes per `Int32`, `UInt32` and `Float32`.**

1. Decision 0030's machinery gets an item size instead of a yes/no: 1, 2 or 4 bytes, or 0 for one 64-bit slot. `Char` stays a slot, as before.
2. The representation stays invisible. Programs read and write such lists as before, and the interpreter keeps its boxed values.
3. Generated code loads and stores items of the packed size. A signed item is sign-extended when it is read, as a slot of a signed type always is.
4. The GPU uploads a packed `[Float32]` as it is, and writes results straight into a new packed list. Measured: the second `map` of the GPU benchmark went from 61 ms to 36 ms.

## Consequences

- A `[Float32]` or `[Int32]` takes half the memory, and twice as many items fit in cache.
- The GPU skips the conversion copies both ways.
- Every place that knew about one-byte packing now asks for the item size: `flat.rs` and the runtime's list code, and both backends' indexing, `push` and set paths. The lists conformance tests cover them on both backends.

## Alternatives considered

- **A separate GPU list type:** it would help only GPU code, and add a type to learn. Packing helps every program with such lists.
- **Converting on upload, as now:** simple, but the copies cost a quarter of a GPU `map`.
