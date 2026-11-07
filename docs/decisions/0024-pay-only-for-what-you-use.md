# 0024. Pay only for what you use

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

Kavach has a growing standard library (JSON, text, files, time, networking, HTTP/1.1 and HTTP/2, TLS, terminal colors, logging, regular expressions) and language features such as capabilities and agents. Not every program needs them: a command-line tool doesn't need HTTP, and a web service doesn't need terminal colors.

Today every program pays for all of it. Measured on 2026-11-01, a program that only prints "hi":

- **compiles 100 functions** where it uses one: all 12 standard library modules are parsed, checked and compiled into every program;
- **is a 4.6 MB executable** (C: about 50 KB, Rust: about 200 KB): every runtime feature, TLS included, is linked in. The cause is one generic runtime entry point, `kv_rt(op, …)`, whose code refers to every feature, so the linker can never leave one out.

That contradicts what Kavach promises infrastructure and embedded users: small, fast-starting executables.

## Decision

**A program pays only for what it uses: at compile time, in the executable, and at run time.**

1. **Standard library modules load on use.** A module is part of a program only when the program (or a module it uses) writes `use std.<name>`. The prelude (spec 10.2) stays small: built-in types, `print`, `debug`, `panic`, `env`, `ask`, `Shared`, `Channel`.
2. **Only reachable functions are compiled.** Native code contains the functions reachable from `main` (and from the tasks and closures it makes), not every function of every module.
3. **Runtime features link on use.** The runtime gets one entry point per feature group (JSON, regex, files, network, TLS, HTTP, terminal, …) instead of the single `kv_rt` dispatcher, so the linker drops whatever a program doesn't call.
4. **Language features cost nothing unused.** Capabilities, agents, contracts and limits are checked at compile time; a program that declares no agent carries no agent runtime.
5. **A budget keeps it so.** A test fails when "hello world" grows past its budget: **under 300 KB**, and a compile of under 0.2 s in a release build of the compiler.

## Consequences

- Smaller executables and faster compiles for most programs; a program that uses HTTPS still gets TLS, and only then.
- `use std.<name>` becomes required where some programs relied on every module being there; the compiler suggests the missing `use`.
- The runtime's entry points change: the code generators (Cranelift and LLVM) and `runtime/native` change together.
- Work: to-do item 3, right after agents.

## Alternatives considered

- **Cargo features to build several runtime variants:** a fixed menu of builds, and a program can't pick exactly what it uses. Linking per feature gives the exact set automatically.
- **Leave it to the linker with link-time optimization only:** it can't see through the `kv_rt` dispatcher, which is the actual cause.
- **Ship the standard library as packages:** right for large, optional parts later (spec 14 says anything that can be a package is one), but the core modules stay built in, loaded on use.
