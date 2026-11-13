# 0103. `std.compress`: gzip, deflate, zstd, brotli and lz4

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Compressed data is everywhere a program talks to others:
- HTTP bodies are sent with `gzip`, `br` or `zstd` when the client's `accept-encoding` allows it. Rath, the web framework, asked for this (to-do item "What Rath needs", (9)).
- Packages travel as compressed archives.
- Games compress their assets and network packets with lz4 or zstd (language request 9 (10)).

Kavach can't do any of it today. Writing the formats in Kavach would be slow and a large job. Each format has a mature Rust crate. A maintainer chose the crates, the API and the limit (2026-11-14).

## Decision

- **Four crates, in `runtime/core`,** so the interpreter and native code give the same bytes:
  - `flate2` on its pure-Rust backend (`miniz_oxide`) for gzip and raw deflate;
  - `brotli` (pure Rust);
  - `lz4_flex` (pure Rust);
  - `zstd` (bindings to the C `libzstd`: the only complete, fast zstd, compressing and decompressing).
- **One pair of functions, with the format as a value**, in a new module `std.compress` (spec 14.21):

  ```kavach
  use std.compress

  const packed = compress.encode(body, Format.Zstd)
  const back = try compress.decode(packed, Format.Zstd)
  ```

  - `compress.encode(data, format, level) -> Bytes`.
  - `compress.decode(data, format, most: 64_000_000) -> Bytes ! CompressError`.
  - A server picks the format at run time from a header's name: `compress.from_name("br")` gives `Format.Brotli`, `none` for a name it doesn't know.
- **Decompressing stops at `most` bytes**, 64 MB unless the call says otherwise, and fails with `CompressError.TooLarge(most)`. A small input that expands to gigabytes (a "zip bomb") can't use up memory.
- No capability: compressing changes nothing outside the program.

## Consequences

- Rath can compress responses, and decompress request bodies with a limit.
- Packages and games can use any of the five formats.
- A program that doesn't use `std.compress` links none of the four (decision 0024).
- `zstd` builds C code, so building Kavach itself needs a C compiler. It already does, for `ring`.
- A future streaming form, for bodies too large to hold in memory, adds functions; the ones above stay.

## Alternatives considered

- **Pure Rust only:** `ruzstd` decodes zstd but can't encode it, so Kavach could read zstd and never write it.
- **Fewer formats now:** brotli and lz4 would wait, though HTTP clients ask for `br` and games for lz4 today.
- **One function per format** (`compress.gzip`, `compress.gunzip`, …): easy to find, but a server choosing from `accept-encoding` would need a `match` to call the right one.
- **A limit written on every call:** always a conscious choice, but easy to get wrong in a hurry, and the default is what most programs want.
