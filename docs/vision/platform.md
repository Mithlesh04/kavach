# The native platform

What Kavach gives natively beyond the core language, why, and in which phase ([decision 0017](../decisions/0017-native-platform-and-speed-targets.md)). Each area gets a spec chapter before its code; this page is the plan, not the rules.

Three principles run through every area:

- **Capabilities everywhere.** Files, network, GPU, processes and the terminal are capabilities (spec 8). A function's signature shows what it can touch, and tests fake it (spec 11.5).
- **Secure by default.** Encryption is never optional. Paths can't escape their folder. Secrets can't be logged (spec 2.11). The unsafe choice is the one that needs extra code.
- **Fast by default.** The fast way is the normal way: asynchronous I/O from the operating system, no copies between buffers, binary messages from types.

## Phases at a glance

| Area | Phase |
|---|---|
| [Files](#files) | 1 (`std.fs`, atomic writes), 2 (scoped `Fs`, io_uring/IOCP, streaming) |
| [Terminal and CLI](#terminal-and-cli) | 1 |
| [Tasks, threads and async I/O](#tasks-threads-and-async-io) | 1 (tasks), 2 (async I/O), 3 (complete) |
| [Logging and remote pipelines](#logging-and-remote-pipelines) | 2 |
| [TCP, UDP and HTTP](#tcp-udp-and-http) | 2 |
| [Talking to existing software](#talking-to-existing-software) | 2 |
| [Kavach Link: native networking](#kavach-link-native-networking) | 3 |
| [Databases and storage engines](#databases-and-storage-engines) | 3 |
| [Games](#games) | 3–4 |
| [Semiconductors and large-scale computing](#semiconductors-and-large-scale-computing) | 3–4 |
| [Infrastructure](#infrastructure) | 2–3 |
| [Processes](#processes) | 3 |
| [GPU compute and pipelines](#gpu-compute-and-pipelines) | 3 |
| [Embedded and microcontrollers](#embedded-and-microcontrollers) | 3 |
| [User interfaces](#user-interfaces) | 4 |

## Files

`std.fs`. The `Fs` capability is **scoped to a folder**: `Fs("./data")` can't reach `../secrets`, and symbolic links that point outside are refused. A function that only reads gets `Fs.read`.

- Writes are **atomic** by default: write to a temporary file, then rename, so a crash never leaves half a file.
- Reads and writes use io_uring on Linux and IOCP on Windows. Large files are streamed or memory-mapped without copying.
- Errors are values (`! FsError`: `NotFound`, `Denied`, `Full`, …), so a missing file is handled, not a crash.

## Terminal and CLI

`std.cli` and `std.term`.

- Command-line arguments are parsed **into a record type**: write `type Args { input: String, verbose: Bool = false }` and get parsing, `--help` text and error messages from the type.
- Colors, progress bars and prompts through the `Console` capability; colors turn off when output is not a terminal.
- Exit codes follow spec 5.8.

## Tasks, threads and async I/O

The model is already in [spec chapter 9](../spec/09-concurrency.md): lightweight tasks, **no `async`/`await`** and no "function colors". Any function can wait on I/O; the runtime runs other tasks meanwhile.

- A work-stealing scheduler runs tasks on one thread per core.
- I/O goes through io_uring (Linux), IOCP (Windows) and kqueue (macOS): one system call for many operations.
- Structured concurrency: a task never outlives the scope that started it (9.2), and there are no data races (9.5).
- Threads are not created by hand; CPU-heavy work uses `spawn`, and the scheduler spreads it over cores.

## Logging and remote pipelines

`std.log`. A log event is a **record type you define**, not a string:

```kavach
type OrderPlaced: LogEvent {
    order_id: Int
    amount: Decimal
    card: secret String       // never written to any log (spec 2.11)
}

log.info(OrderPlaced(order_id: 7, amount: 499, card: c))
```

- Levels, sampling and per-module filters are set in `kavach.toml`, not in code.
- Sinks: the console, files with rotation, and **remote pipelines**: OpenTelemetry (OTLP) for existing tools, and Kavach Link for other Kavach services. A slow sink never blocks the program; events are batched in memory and dropped with a counter if a limit is reached.
- The agent audit log (spec 8.6) is a log stream with stronger guarantees.

## TCP, UDP and HTTP

`std.net`, behind the `Net` capability, which can be limited to hosts (`Net("api.stripe.com")`).

- TCP and UDP sockets, HTTP/1.1, HTTP/2 and HTTP/3 clients and servers.
- TLS 1.3 is on by default; plain connections need an explicit, visible opt-out. Certificates are checked; there is no "accept any certificate" switch outside tests.
- Timeouts are required: every network call has a deadline (spec 9.6), so a hung server can't hang the program.

## Talking to existing software

Programs already in use (Node.js, Java, Go, Python, C#, …) must be able to call Kavach programs and be called by them:

- **HTTP/JSON and gRPC**, generated from Kavach types: a `pub` function can be served as an HTTP or gRPC endpoint, and a Kavach program can call any HTTP or gRPC service.
- **Kavach Link client libraries** for Node.js, Java, Go and Python, so those programs get the fast, typed protocol too.
- **A C library from any Kavach package** (`kavach build --lib`): a C ABI that Node-API, JNI, cgo, P/Invoke and Python's ctypes can load, for calls inside one process.
- **Secure by default:** TLS and mutual authentication are on. A plain, unencrypted connection is possible, for example on localhost or inside one machine, but only when the code says so explicitly, so it is visible in review.

## Kavach Link: native networking

The pain points of networking between services today:

| Pain point | Why it hurts |
|---|---|
| Connection setup | TCP + TLS cost 2–3 round trips before the first byte; across regions that is hundreds of milliseconds |
| Head-of-line blocking | One lost TCP packet stalls every request on the connection |
| JSON everywhere | Encoding and parsing text is often the largest CPU cost of a service, and it loses types |
| Copies | Data is copied from the kernel to a buffer, to a string, to an object |
| Security is optional | Plain HTTP inside the data center, shared API keys, no check of **what** a caller may do |
| Schemas drift | Client and server types are written twice and break silently |

Kavach Link is a protocol for Kavach services to talk to each other, built only from proven parts:

- **Transport: QUIC** (as in HTTP/3): encrypted always, 1 round trip to connect and 0 to resume, no head-of-line blocking between streams, and survives a change of network.
- **Identity: mutual authentication**: both sides prove who they are with certificates or keys. No shared secrets.
- **Messages from types**: a `pub` function or capability of a service is callable remotely; its Kavach types define a compact binary encoding, so there is no JSON and no hand-written schema. Changing a type is checked like any API change (`kavach api check`, spec 12.5).
- **Permission per call**: the caller's capabilities travel with the connection, so a service can say "this client may call `refund` up to 10,000" with the same `limit` rules agents use (spec 8.3).
- **Zero copy**: messages are read from the receive buffer in place.

It is not new cryptography and not a replacement for HTTP on the open web: it is the fast, safe default between Kavach programs, with HTTP and gRPC gateways for everything else.

## Databases and storage engines

Database companies need a language that is fast at the lowest level and safe at every level, because a storage bug loses data. Kavach aims to be a language such teams can adopt for a whole engine:

| Need | What Kavach gives |
|---|---|
| Control over the disk | Direct I/O, explicit `fsync` and write ordering, memory-mapped files, preallocation, all through io_uring/IOCP |
| Data integrity | Checksums (CRC32C, xxHash) in `std`, torn-write detection, atomic file replacement (already in `std.fs`) |
| Speed of C | Native code with no GC and no pauses; SIMD; atomics and lock-free structures; custom allocators and arenas (spec 6.3); zero-copy binary formats from types |
| Predictable latency | No garbage collector; allocation visible and controllable; tasks that never block a core |
| Memory safety | No use-after-free, no data races (spec 9.5); `unsafe` only where opted in and audited (`kavach audit`) |
| Security | Encryption at rest and in transit, secrets that can't be logged (spec 2.11), capabilities that limit what each part of the engine can touch |
| Proof it is correct | Contracts and invariants on every page and record format; generated tests (spec 11); deterministic simulation of crashes, disk faults and network splits |

Data structures such as B-trees and LSM trees live in packages, not in `std`, but the language and `std` give them everything they need.

## Games

Game studios need every frame on time: a 16 ms budget with no surprise pauses. Kavach's plan for them:

| Need | What Kavach gives |
|---|---|
| Steady frame times | No garbage collector; arenas reset every frame (spec 6.3); allocation that is visible in code |
| Many entities, fast | Data layouts that keep fields of many entities together (struct of arrays), SIMD vector and matrix math in `std` |
| Graphics, audio, input | GPU rendering through the same layer as GPU compute (`wgpu`); audio and input through capabilities |
| Multiplayer | UDP and Kavach Link; deterministic simulation (fixed-point math, seeded randomness) for lockstep and rollback netcode |
| Fast iteration | Hot reload of game code while the game runs; the interpreter for quick tries (spec 10.9) |
| Mods without risk | Mods are Kavach code limited by capabilities: a mod can't touch files or the network unless the game allows it |

## Semiconductors and large-scale computing

Semiconductor companies (chip design, verification, simulation and fab data) work with billions or trillions of items: netlists with billions of gates, simulation waveforms, wafer and test measurements, and math that runs for days. They need the speed of hand-tuned C++, floating-point results that are accurate and exactly repeatable, and code that is safe to change. Kavach's plan for them:

| Need | What Kavach gives |
|---|---|
| Data larger than memory | Streaming and out-of-core processing; memory-mapped files; columnar layouts (Arrow, Parquet) through packages |
| Compact data | Sized integers (spec 2.1), bit vectors and bit-packed lists, struct-of-arrays layouts: less memory per item means less time waiting for it |
| Math throughput | SIMD (AVX2, AVX-512, NEON) and all cores for loops over big data, with results identical to the plain loop (to-do item 1); GPU offload (GPU compute) |
| Accurate floating point | Every operation correctly rounded as IEEE 754 requires, and never reordered behind the program's back (no "fast math" unless asked); compensated sums (Kahan, Neumaier) and exact dot products in `std.math`; `Float128` or double-double for more digits; explicit rounding modes; interval arithmetic that bounds the error; exact `Decimal` (spec 2.1) where base 10 matters |
| Repeatable results | Deterministic reductions: a floating-point sum gives the same bits on every run and machine unless the program asks for a reordered, faster one |
| Numbers chip work needs | 128-bit and arbitrary-precision integers (`BigInt`), fixed-point, complex numbers, intervals; linear algebra, sparse matrices and FFT in packages |
| Huge graphs | Compact graph storage for netlists and timing graphs; bit-parallel logic simulation |
| Long jobs on many machines | Checkpoint and restart (durable execution, Phase 2), work split across a cluster over Kavach Link |
| Existing tools | Calling C, C++ and Python (NumPy) libraries; reading design formats (Verilog, LEF/DEF, GDSII/OASIS) through packages; TCL-style scripting replaced by typed Kavach scripts |
| Proof it is correct | Contracts and generated tests (spec 7, 11) on every algorithm; benchmarks on billions of items to prove the speed |

## Infrastructure

Cloud, networking and DevOps companies run software for years on thousands of machines. What they need, and what Kavach gives:

- **Easy to ship:** one static executable (`kavach build`), fast start, small memory, cross-compiling for every target from one machine, small container images.
- **Long-running services:** graceful shutdown on OS signals, timeouts everywhere (spec 9.6), no memory growth from leaks (reference counting with no cycles in values, spec 6).
- **Observability:** logs as typed records, metrics and traces, exported with OpenTelemetry (see [logging](#logging-and-remote-pipelines)).
- **Plugins and extensions:** loaded as sandboxed Kavach code, limited by capabilities.
- **Supply-chain security:** signed packages, a bill of materials for every build, and `kavach audit` listing unsafe code and capability use in every dependency.

## Web backends

Backend developers who build web services and APIs today with Node.js (Express, Fastify), Python (Flask, FastAPI, Django) or similar. What they need, and what Kavach gives:

- **Routes from functions:** a handler is an ordinary function; its parameters and return type define the request and the response. Path, query and JSON body are parsed and checked from the types, so a bad request gets a clear 400 without any validation code.
- **Middleware** (logging, authentication, CORS, rate limits, compression), written as plain functions and composed in order.
- **Databases:** drivers for PostgreSQL, MySQL and SQLite, connection pools, migrations, and queries checked against record types; no SQL built by joining strings.
- **The web's everyday parts:** cookies and sessions, file uploads, static files, server-side templates (escaped by default), WebSockets and server-sent events, background jobs.
- **API contracts:** an OpenAPI description generated from the handler types, and typed clients for other services.
- **Fast feedback:** `kavach run --watch` restarts on every save; handler tests call a function, not a running server.
- **Safe by default:** HTTPS, secrets never logged (spec 2.11), capabilities saying which handlers may reach the database or the network (spec 8.1).
- **Easy to ship:** one executable, fast start and small memory, as for [infrastructure](#infrastructure).

## Processes

`std.process`, behind a `Process` capability that lists the programs a function may run. Arguments are passed as a list, never through a shell, so there is no command injection. Output can be streamed. Multi-process programs talk through Kavach Link over local sockets.

## GPU compute and pipelines

`std.gpu`, behind a `Gpu` capability.

- Kernels are Kavach functions marked `gpu fn`, using a safe subset of the language (no heap allocation, no capabilities). They are compiled to SPIR-V and run through Vulkan, Metal and DirectX 12.
- Buffers are typed (`GpuBuffer<Float32>`), with bounds checks in debug builds, so a kernel can't read outside its data.
- A pipeline is a typed chain of stages. The compiler checks that each stage's output type matches the next stage's input.
- Data stays on the GPU between stages; copies to and from the CPU are explicit.

## Embedded and microcontrollers

A `no_heap` build profile for ARM Cortex-M and RISC-V, through the LLVM backend:

- no heap: only fixed-size values and arenas with a fixed size (spec 6.3), so memory use is known at compile time,
- no runtime threads: tasks run on a small single-threaded scheduler, or interrupts,
- hardware registers through `unsafe` blocks with typed wrappers in a board package.

Contracts and generated tests work the same; tests run on the development machine.

## Operating systems and hardware

Companies that build operating systems, drivers, firmware and hardware tools need direct control. Kavach gives it without giving up safety: low-level code is `unsafe` only where it says so (spec 6.4), and every device or system resource is a capability, so a reviewer sees exactly what a function can touch.

- **The operating system's everyday APIs**, each behind a capability: the clipboard (`Clipboard`: read and write text, images, files), notifications, file dialogs, the system tray, power and screen state, signals, services and daemons (systemd, Windows services, launchd), users and permissions.
- **Low-level system APIs:** system calls, memory mapping, raw file descriptors and `ioctl`, process and thread control, Win32 and macOS frameworks through C interop with typed wrappers.
- **Hardware:** USB, serial ports, Bluetooth, GPIO, I2C and SPI, PCI devices, DMA buffers, and memory-mapped registers with volatile reads and writes through typed wrappers.
- **Kernels and bare metal:** a freestanding profile (no operating system, a custom allocator or none), custom entry points and linker scripts, interrupt handlers, inline assembly, and drivers and kernel modules written in Kavach.
- **Why Kavach fits:** values never alias, there is no GC pause, latency is predictable, and `kavach audit` lists every `unsafe` block and every capability a package uses.

## User interfaces

`std.ui` (Phase 4): a declarative toolkit where a screen is a function from state to a view, rendered on the GPU.

- Desktop first (Windows, Linux, macOS), then the web through WASM, then mobile.
- Layout, text and accessibility are built in, not optional.
- State changes go through typed messages, so UI logic is tested like any other code (spec 11).
