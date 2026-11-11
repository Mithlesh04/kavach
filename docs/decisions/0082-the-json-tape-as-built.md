# 0082. The JSON tape as built

- **Status:** Accepted
- **Date:** 2026-11-11

## Context

[Decision 0032](0032-json-on-a-tape.md) put each parsed JSON document on one buffer, a tape, with `Json` values pointing into it. A maintainer asked (2026-11-11) for it to be built now, before the SIMD work. Building it met three facts 0032 did not settle: the parser hands values over bottom-up (a container after its items); `Json` is a union whose `Array` and `Object` hold a real list and map that programs match and keep; and native code reads a union's fields through the runtime whenever an object is not a plain fixed object (as for errors with `context`).

## Decision

- **The tape is a list of 64-bit words per document** (`runtime/native/src/rt_json_tape.rs`). Every value is one word with its kind in the top 4 bits. A string or number is its place and length in the parsed text, which the document keeps a copy of, or in a side text for strings with escapes. An array or object is the index of its run: its length, then its items, or its keys and values in turn. Runs are written as the parser closes each container, so no skip links are needed, and item `i` of an array is one index away.
- **A `Json` array or object from a document is a small object pointing into it** (`Data::Json`): the document (shared, as it never changes, so tasks can share it too), the value's word, and its list or map, made the first time a program matches it and then kept. Its header holds its variant, so `match` needs no change in the compiler. Numbers, text, booleans and `null` are made as ordinary values when read.
- **Reads skip the list and the map** (`runtime/native/src/rt_json_read.rs`): `j[key]`, `j[i]` and the methods `list`, `object`, `int`, `float`, `text`, `bool`, `is_null` and `len` are runtime operations in native code. `j["key"].int() ?? 0` (and `float`) with a constant key reads the number in place and makes nothing, and each constant key remembers where it was found, as objects in one array tend to list fields in the same order.
- **The interpreter keeps plain union values,** as 0032 said; the conformance and native tests check that both behave alike.

## Consequences

- On the [JSON benchmark](../../benchmarks/json/README.md) (Linux, against the commit before): `parse` 504 → 198 ms, `decode` 382 → 357 ms, `read` 100 → 108 ms, peak memory 0.70 → 0.36 GB.
- A `Json` value kept after parsing keeps the whole document alive, as 0032 expected.
- What `read` still spends is mostly making a small object for each array item a loop visits. The SIMD structural pass of 0032 comes next.

## Alternatives considered

- **simdjson's layout, with skip links:** needs a parser that writes containers before their items; ours closes them after, and runs with item indices give `j[i]` at once.
- **Making numbers and text point into the tape too:** a `match` on them needs the text as a value anyway, and the fused reads above already skip making them.
