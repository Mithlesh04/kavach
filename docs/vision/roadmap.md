# Roadmap

Each phase ends with a clear **exit test**. We move on only when it passes. No dates are promised until Phase 1 is underway.

## Phase 0: Design (now)

- Language spec v0.1 ([docs/spec](../spec/README.md))
- Example programs that read well on paper ([examples](../../examples/README.md))
- Decision records for every major choice
- Feedback from 20 to 30 target developers on the spec and examples (GitHub issue forms: first impression, spec feedback, use case)
- Conformance tests for chapters 1–7, written against the spec
- VS Code syntax highlighting, so examples are easy to read

**Exit test:** target developers read the refund-bot example and say "I would try this."

## Phase 1: Core language ("Hello, Kavach")

**Status (2026-10-31): the exit test passes.** Examples 01–04 run and pass `kavach test` with no hand-written test files, and the conformance tests for chapters 1–7 (and the rest of what is built: modules, generated tests, limits, capabilities, the standard library) pass on Windows and Linux (x86-64). Programs run with the development interpreter ([decision 0014](../decisions/0014-development-interpreter.md)) and compile to native code: every conformance test and example builds natively, through LLVM for `kavach build` ([decision 0018](../decisions/0018-llvm-backend-through-llvm-tools.md)) and Cranelift for `kavach run --native`, checked against the interpreter and for leaks. Native code is faster than C++ on every benchmark but one, where it is level ([results](../../benchmarks/loops/README.md#results)). The standard library's parts that need no operating system are written in Kavach (`std.text`, `std.collections`, `std.math`, `std.json`, `std.fs`'s `FsError`), so **every item of Phase 1 is done**. Scoped `Fs` values moved to Phase 2, with the other file work.

- Compiler: lexer, parser, resolver, type checker, Cranelift and LLVM backends
- Built-in capabilities (`Console`, `Fs`, `Net`, …) checked at compile time
- Errors, contracts (runtime checks), automatic memory
- Generated testing: `example` clauses, inputs generated from types and contracts, shrinking, found cases, crash fix suggestions
- File length limits (`[limits]`)
- Token-saving tools for AI: `kavach outline`, `kavach query`, `--format json` messages (spec 13)
- Standard library: `core`, `collections`, `text`, `math`, `time`, `io`, `test`, `regex`, `json`; files with atomic writes; CLI arguments parsed into records ([platform](platform.md))
- CLI: `kavach new | run | build | test | fmt`
- Platforms: Linux and Windows (x86-64)

**Exit test:** examples 01–04 run and pass `kavach test` with no hand-written test files; conformance tests for chapters 1–7 pass.

## Phase 2: Agents (the "wow" release)

**Status (2026-11-01):** agents work end to end, in the interpreter and in native code: capabilities, `can`, `limit`, `approve`, `ask<T>` with `std.ai`, the modes, `--dry-run`, the audit log, secrets, and limit totals kept across restarts ([refund example](../../examples/05-refund-bot/README.md)). Durable handlers work in both too: a journal, and events a crash cut short finish when the agent starts again ([decision 0025](../decisions/0025-agents-resume-by-replaying-a-journal.md)). Approvals that wait across restarts are still open.

- `capability`, `agent`, `limit`, `approve`, `ask<T>`
- Agent modes `observe | suggest | enforce`, `--dry-run`, audit log, secrets
- Durable execution, so a handler can pause for a human approval and resume
- Automatic capability fakes and agent exploration in `kavach test`
- `std.json`, `std.net` (TCP, UDP, HTTP/1.1–3, TLS by default), `std.ai`
- `std.log`: structured logs as records, OpenTelemetry export
- Async I/O: io_uring, IOCP and kqueue under the task scheduler
- C interop
- Language server + VS Code extension
- AI tooling ([spec 13](../spec/13-ai-tooling.md)): `kavach context`, MCP server, `llms.txt`, errors with fixes, the index
- Browser playground on the docs site (https://mithlesh04.github.io/kavach/)

**Exit test:** a developer who has never seen Kavach builds a guarded agent in under 30 minutes. We run one agent of our own in production.

## Phase 3: Production

- LLVM backend for optimized release builds
- Python and JavaScript interop
- Package manager and registry
- Concurrency complete: tasks, channels, `Shared<T>`
- Kavach Link: typed, encrypted service-to-service protocol over QUIC; `std.process`
- GPU compute (`gpu fn`, typed pipelines); `no_heap` embedded profile
- Large codebases: workspaces, `--changed` builds and tests, `[architecture]` rules, `kavach api check`, `deprecated … fix`
- Test report with weak spots, `kavach test --suggest`, `kavach audit --panics`
- `kavach edit` and `rename`; the token benchmark published (spec 13.10)
- Basic static checking of contracts
- Public benchmarks against C#, Go, Rust, Python
- Operating system APIs behind capabilities (clipboard, notifications, signals, services) and device access (USB, serial, GPIO, I2C, SPI)
- macOS, ARM64

**Exit test:** at least 3 outside companies run Kavach in production.

## Phase 4: Reach

- WASM target (browser and edge)
- `std.ui`: native UI toolkit, desktop first, then web and mobile
- Android (NDK), RISC-V
- Kernels and bare metal: a freestanding profile, interrupt handlers, inline assembly, drivers and kernel modules
- Generated plans: agents emit Kavach code that is verified before it runs
- Static contract proving (SMT solver) for critical code
- Security audit by an outside firm

## v1.0: Stability promise

- Spec frozen for edition 1; backward compatibility promise from here on
- Changes after this only through [editions](../spec/10-modules-and-packages/08-editions.md)
- Governance moves to an independent foundation
