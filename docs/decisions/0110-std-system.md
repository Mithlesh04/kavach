# 0110. `std.system`: the machine, basics free, details behind `System`

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

A program can't ask what machine it runs on: its system, processor, cores, memory or GPUs. It needs them to size its work (threads, batches, whether a GPU pays off), for bug reports and benchmark headers, and for runs that must say where they happened (Sakshi, an evidence harness for chip design, records this with each run). A maintainer asked for it on 2026-11-15 (to-do item 13).

Some of these facts identify nobody: every machine of a kind has the same system name and core count. Others, taken together (the processor's model, the GPUs, the memory, the hostname, the uptime), can tell one machine from another, which is what fingerprinting does. Reading them needs system calls safe Rust can't make on Windows and macOS.

## Decision

- **A module, `std.system`** (spec 14.25), written in Kavach over a few runtime functions that give numbers and text, as `std.process` is.
- **The basics are free**, like reading the clock: `system.os()`, `system.arch()` and `system.cores()`. **The rest needs `uses System`**, a new built-in capability (8.1): `memory`, `cpu`, `gpus`, `hostname` and `uptime`. `kavach audit` lists it, and an agent gets it only when a person approves. A maintainer chose this split (2026-11-15).
- **Nothing fails:** what a system doesn't say is `""` or 0.
- **No new outside dependency.** Linux answers from `/proc` with no `unsafe`. Windows and macOS need their own calls, made through `windows-sys` and `libc`, which `runtime/process` already uses for `std.process` ([decision 0105](0105-std-process.md)); so the machine's facts live there too, each `unsafe` call with a `SAFETY:` comment. The GPUs come from `wgpu`'s list of adapters, which opens no device.
- `process.cpu_ms()`, the program's own processor time, joins `process.peak_bytes()` (14.23), free to read.

## Consequences

- A program can size its threads and batches from `system.cores()` with no capability, and a benchmark can print its machine with `uses System`.
- `runtime/process` now holds two kinds of system calls: running programs and reading the machine. Both are small and checked by the same review.
- `system.gpus()` has no memory size, as `wgpu` doesn't report it. Each vendor's own interface would be needed; that waits for multi-GPU work (to-do item 16).

## Alternatives considered

- **Everything free.** Simpler, but a program, or a dependency, could fingerprint the machine without anyone seeing it in the code.
- **Everything behind `System`.** A program sizing its threads would need a capability, and every library that does so would ask for it, so people would grant it without reading.
- **The `sysinfo` crate.** It covers all of this, but it is a large outside dependency for a few calls we can make ourselves, and it reads much more than we need.
