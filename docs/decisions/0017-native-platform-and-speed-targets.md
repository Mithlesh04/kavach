# 0017. A native platform, and speed targets we can prove

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Kavach's first audience is teams running AI agents in production, and its second is backend and systems developers ([vision](../vision/vision.md)). Both need more than a core language: files, networking, logging, many cores, the terminal, and soon GPUs, small devices and user interfaces. If these come from packages of mixed quality, they bring back exactly the problems Kavach exists to remove: hidden side effects, missing security checks and slow defaults.

The request was to give all of this **natively**, and to be at least 80–100% faster than the languages people use today: Java, C#, Go, Node.js, Ruby, Gleam, Roc, Zig, Rust and C++.

## Decision

1. **Kavach ships a native platform**, designed together with the language and built on capabilities (spec 8), so every side effect is visible in signatures and can be faked in tests (spec 11.5). The areas, and the phase each arrives in, are in [vision/platform.md](../vision/platform.md):
   files, terminal and CLI, tasks and threads, async I/O, structured logging with remote pipelines, TCP/UDP/HTTP, a Kavach-native network protocol, processes, GPU compute, embedded devices and a UI toolkit.
2. **No new cryptography.** Security comes from proven building blocks (TLS 1.3, QUIC, Noise) used correctly by default: encryption is never optional, and the unsafe choice is the one that needs extra code.
3. **Speed targets are measured, not claimed.** We publish a benchmark suite (roadmap Phase 3) and treat a regression as a bug. The targets:

   | Compared with | Target |
   |---|---|
   | Ruby, Node.js, Python | At least 2× faster on CPU-bound work; usually far more |
   | Java, C#, Go, Gleam, Roc | At least 80% faster on allocation-heavy work and startup; lower memory and no GC pauses; match or beat throughput |
   | Rust, C++, Zig | Match them (within 10%) on CPU-bound work. Beat them where their **default** libraries are slow: I/O through io_uring or IOCP, zero-copy networking, typed binary messages instead of JSON, arena allocation |

4. **The UI toolkit and embedded targets are in scope**, changing the vision's "not now" for UI. They come after the core platform (Phase 4), and the vision is updated to say so.

## Consequences

- The standard library grows: `std.fs`, `std.cli`, `std.term`, `std.log`, `std.net`, `std.process`, `std.gpu`, `std.ui`. The rule that it stays small still holds for everything else.
- "80–100% faster than Rust, C++ and Zig" is **not** a goal. Those languages compile to near-optimal machine code; a program can't be 80% faster than the best machine code for the same work. Saying otherwise would cost trust the first time someone ran a benchmark.
- Each area gets a spec chapter before its code (AGENTS.md section 4).

## Alternatives considered

- **Leave platform libraries to packages:** fast to start, but every package would choose its own security and speed defaults, and capabilities would not cover them.
- **Bind existing C libraries for everything (libuv, OpenSSL, Qt):** quick, but it brings C's memory bugs into a memory-safe language and ties speed to their defaults. Kavach uses the operating system directly where it can, and well-reviewed libraries (rustls, quinn, wgpu) where it must.
