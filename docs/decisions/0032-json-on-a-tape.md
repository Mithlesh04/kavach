# 0032. JSON on a tape: one buffer per document

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

`Json` (spec 2.12) is stored today like any union: one heap object per
value. Parsing 52 MB makes millions of small objects, and reading
fields chases pointers — the [JSON benchmark](../../benchmarks/json/README.md)
shows parsing only as fast as Python's C parser and reads 1.4× slower.
simdjson (C++) parses at several GB/s, and its speed starts with a
compact layout, not with SIMD: one flat buffer per document that values
point into. To-do item 1 names this change, and says it gets a decision
record first because every place that matches a `Json` changes.

## Decision

**A parsed document is one buffer — a tape — and a `Json` value points
into it.** The spec's union stays exactly as written; this is the
native representation, not a language change.

1. **The tape** is an array of 64-bit words. Each value is a tagged
   entry: null, true and false are one word; numbers carry their value
   inline; strings carry offset and length into the document's text,
   which the document keeps alive (no copy); arrays and objects carry
   their length and a skip link to their end, so stepping over a value
   is one jump however deep it nests.
2. **A `Json` value** is one small object: a reference to the document
   and a tape index. Sub-values (`j["items"][0]`) share the document;
   the document is freed when the last value into it goes.
3. **`match` on `Json`** reads the tag at the index. `list()` and
   `object()` walk entries behind the skip links instead of building
   lists and maps — `decode<T>` and field reads never allocate.
4. **Building** `Json` in a program writes a builder tape the same
   stringify walks linearly; text pieces land in one growing buffer.
5. **The interpreter keeps plain union values.** It is the semantics
   reference; the conformance suite proves the tape behaves identically.
6. **SIMD comes after the tape**, as its own change: structural
   indexing (quotes, braces, commas) classifies 64 bytes per step and
   feeds the tape writer. The tape is what makes that worth doing.

## Consequences

- `rt_json.rs`, `rt_json_decode.rs`, `rt_json_build.rs`, `rt_json_pull.rs`
  and the KIR lowering of `match` on `Json` switch to tape accessors; a
  contained but wide change, gated by the existing JSON conformance
  tests and the benchmark.
- Memory per document drops by roughly 10×, and `read` becomes index
  arithmetic on one cache-friendly buffer.
- A `Json` value kept after parsing holds the whole document alive.
  That is the trade simdjson makes too; a program that wants one small
  piece can `decode` it into its own records.

## Alternatives considered

- **Pooling the per-value objects:** fewer allocations, same pointer
  chasing; the read path stays slow.
- **Binding simdjson through C interop:** fast parsing, but a foreign
  dependency in the standard library, no help for building or
  stringify, and the interpreter could not mirror it.
