# 0123. Tracy for live profiling, and the GPU's time in traces

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

A game must see where each frame's time goes, on the CPU and on the GPU, while it runs (to-do 9 (13)). `KAVACH_TRACE` already writes every span to a Chrome trace file (spec 14.12), read after the program ends. Two things were missing: the GPU's own time for its work, and a live view. Tracy is the profiler most game developers use: a viewer that shows each thread's zones, frame by frame, as the program runs. A maintainer chose on 2026-11-16, from options with a recommendation: GPU spans in the Chrome trace, and live viewing in Tracy through the `tracy-client` crate.

## Decision

- **The GPU's time** comes from timestamp queries, which most GPUs have (`wgpu`'s `TIMESTAMP_QUERY`). With a trace on, a frame's drawing and each GPU `map` write the GPU's clock before and after their work. The time between is shown on a row of its own, `GPU`. The GPU's clock isn't the CPU's, so the event starts where the CPU sent the work. A GPU without timestamps shows no GPU row.
- **Tracy** comes through the **`tracy-client`** crate (MIT or Apache-2.0), which builds Tracy's own C++ client (BSD-3-Clause). It is linked only into what asks for it: `kavach run --tracy` and `kavach build --tracy`. A program built without it doesn't link Tracy (decision 0024). The client listens only on the machine itself (`localhost`).
- Each span is a Tracy zone, begun and ended where the span is, through the crate's safe calls. Zones nest on a thread: a span that ends while another task's span on the same thread is open stays open in Tracy until that one ends. The GPU's timed work goes to a Tracy GPU context.
- The Chrome trace and Tracy can be on together.

## Consequences

- A game's frames can be watched live, CPU and GPU, with no code beyond `log.span`.
- Building Kavach needs a C++ compiler for Tracy's client. Kavach's builds already need a C compiler and a linker.
- A program built with `--tracy` is larger (Tracy's client), and spends a little time on each span.
- Tracy's *fibers*, one row per task, would show tasks that take turns on a thread exactly. They need `unsafe` calls into Tracy's C functions, which this crate may not make (AGENTS.md section 8); they wait for a decision of their own.

## Alternatives considered

- **The Chrome trace only:** no new dependency, but nothing live. Rejected by the maintainer's choice.
- **Tracy only, in place of the Chrome trace:** a Chrome trace needs no viewer to install, and its file is kept. Rejected: both are useful.
- **Tracy's protocol written ourselves:** it changes with each Tracy version. Rejected.
