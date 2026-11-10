# 0081. Packed executables

- **Status:** Accepted
- **Date:** 2026-11-10

## Context

A maintainer asked (2026-11-10) for `kavach build` to write smaller executables, compressed with nothing lost. They are already stripped and linked without unused code (decision 0024): hello world is 500 KB on Windows and 579 KB on Linux, mostly the runtime's code. Compressing the whole file leaves 34–47% of it, but then the file must unpack itself when it starts. Linux could run the unpacked program from memory; macOS and Windows can't, and on Windows a program can't replace itself with another one either.

## Decision

A maintainer chose (2026-11-10), step by step as the measurements came in:

- **Asked for, not by default:** `kavach build --compress` packs with deflate (`miniz_oxide`), `--compress=max` with xz (`lzma-rust2`; `lzma-rs` was the first choice, but its encoder doesn't compress: the file stayed at 100%). Plain `kavach build` is unchanged. Both libraries are pure Rust and new outside dependencies of `runtime/unpack`, which `kavach` uses to pack.
- **A starter written in C:** a packed file is the starter, then the compressed program, then a 40-byte trailer: `KVPACK01`, the method, the compressed and unpacked sizes, and a 64-bit FNV-1a hash of the unpacked program. The starter is C (`runtime/unpack/starter/starter.c`) with two vendored decoders, tinf (deflate, zlib licence) and xz-embedded (xz, 0BSD), in `runtime/unpack/vendor/`: 35 KB on Windows and 23 KB on Linux. A Rust starter was measured first: Rust's standard library alone made it 204 KB on Windows and over 300 KB on Linux, more than a small program saves.
- **Built with Kavach, carried inside it:** `runtime/unpack/build.rs` compiles the starter with the C compiler Rust's builds need (MSVC on Windows, `cc` elsewhere), and `kavach` embeds it, so an install (decision 0080) has no extra file.
- **Unpacked once, into a cache folder,** on every system alike: the copy is named by its hash and size, written under a temporary name and renamed, so programs started at once never see half a file. Later starts check the name and size and run the copy at once. On Linux and macOS the starter replaces itself with the copy (`execv`), so the process, its signals and its exit code are the program's own; on Windows it starts the copy with the same command line, waits, and ends with its exit code, ignoring Ctrl+C itself so that only the program decides how to stop. The folder is spec 10.5's (`KAVACH_UNPACKED`, else the system's cache folder).
- `kavach version` unpacks a packed file in memory to read what built it.
- Vendored third-party code sits in `vendor/` folders, which the repository check exempts from the 500-line limit (AGENTS.md section 3).

## Consequences

- Measured on a 1.1–1.3 MB program: `--compress` leaves 42–48% of the size, `--compress=max` 34–40%, starter included. Every run of the packed program printed the same output, arguments, panic message and exit code as the plain one, on Windows and Linux (`tools/cli/tests/packed.rs`).
- The first start unpacks (milliseconds for deflate, tens for xz) and leaves a copy in the user's cache folder; later starts cost a check of one file's size.
- Some antivirus tools distrust self-unpacking programs, which is why packing is opt-in; the spec says so.
- C code in the repository for the first time, and two vendored decoders to update by hand; no Rust `unsafe` was needed.
- macOS is written but not yet tested (no Mac here).

## Alternatives considered

- **A Rust starter:** safe and simple, but 204–300+ KB, so small programs would have grown on Linux.
- **A Rust starter without the standard library:** small, but every system call in `unsafe`, a wide exception to AGENTS.md section 8.
- **Running from memory on Linux (`memfd_create`, `fexecve`):** no file written, but Linux only; the cache folder works everywhere and makes later starts as fast as an unpacked program.
- **Packing by default, or on Linux and macOS only:** smaller files for everyone, but slower first starts and antivirus warnings nobody asked for.
- **An existing packer (UPX):** a separate tool to install, often flagged by antivirus tools, and its stub isn't ours to check.
