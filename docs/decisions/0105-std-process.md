# 0105. `std.process`: other programs as trees, through our own system calls

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Programs need to run other programs: a compiler, a simulator, `git`, a tool with no library. Sakshi, an evidence harness for chip design written in Kavach, wraps tools such as Verilator and Yosys, so it can't work without this. The roadmap had `std.process` in Phase 3; a maintainer moved it ahead, to be built next (2026-11-14).

What makes running a program hard is ending it. A tool often starts more programs of its own. When a timeout fires or a task is cancelled, killing only the first one leaves the rest running. Every system has a way to hold a program and everything it starts: a job object on Windows, a process group on Linux and macOS. Both take system calls that safe Rust can't make. So do a finished program's processor time and peak memory, which Sakshi needs for its budgets.

## Decision

- **A module, `std.process` (spec 14.23).** `process.run(program, args)` runs a program to its end and gives its exit code, its output and what it cost. `process.start` gives it back running, to read its output line by line as it comes. Arguments are a list and never pass through a shell.
- **A capability, `Process` (spec 8.1).** It can name one program, `uses Process("verilator")`, checked when the program runs, as `Fs("folder")` is.
- **A program is a tree.** `kill()`, a `Child` whose last holder ends, a cancelled task and a time limit all end the program and everything it started.
- **Our own system calls, in a new crate, `runtime/process`, allowed `unsafe`** (a maintainer's choice, 2026-11-14), as `runtime/native` is (decision 0016). Both the interpreter and native code use it. It calls the systems through `windows-sys` and `libc`, which Kavach already depends on, so no outside dependency is added. Each `unsafe` block has a `SAFETY:` comment.
  - Windows: the program starts suspended, joins a job object that kills everything in it when it closes, and only then runs. The job gives processor time and peak committed memory for the whole tree.
  - Linux and macOS: the program starts in a process group of its own, `killpg` ends the group, and `wait4` gives processor time and the largest resident set.

## Consequences

- Kavach programs can drive any tool, with no shell and so no command injection.
- A timeout or a cancel never leaves part of a tool running.
- Kavach has three places with `unsafe` now: `runtime/native`, one call in `runtime/gpu` (decision 0077), and `runtime/process`. AGENTS.md lists them.
- The memory figure means slightly different things per system (committed memory on Windows, resident memory elsewhere). The spec says so.
- Text sent to a program's input, output as `Bytes`, a `kavach test` fake and resource limits are open questions in spec 14.23.

## Alternatives considered

- **`process-wrap` and `sysinfo`:** no `unsafe` of our own, but two new dependencies. `sysinfo` is large and samples memory now and then, so it can miss the peak.
- **`process-wrap` alone:** it kills the tree, but gives no processor time or peak memory, which Sakshi needs.
- **Killing only the first program:** simple, but a timeout would leave the tools it started running.
