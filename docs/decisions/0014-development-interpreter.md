# 0014. A development interpreter before the native backend

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

The front end (lexer, parser, resolver, type checker) was done, but no program could run. The planned next step was the Cranelift backend. That is a large piece of work, and it needs outside crates. Meanwhile, nothing checked that programs actually *behave* as the spec says: the conformance tests only checked compile errors.

## Decision

Build a small tree-walking interpreter first (`compiler/interp`), and the `kavach` command on top of it (`tools/cli`):

- `kavach run file.kav`, `kavach test file.kav` and `kavach check file.kav` work now.
- The conformance runner (`cargo test -p kavach-cli`) checks every conformance test's output, exit code and panic message, and runs examples 01–04 and their `example` clauses.
- The interpreter uses the checker's results, such as the type of every number literal, so it behaves exactly like compiled code will.

Native compilation stays the way programs ship ([decision 0005](0005-backends-cranelift-llvm-wasm.md)). Later, the interpreter also powers `kavach repl` and `kavach eval` (spec 10.9), where instant start matters more than speed.

## Consequences

- The Phase 1 exit test can be checked today: conformance tests for chapters 1–7 pass end-to-end.
- Run-time rules are tested now: overflow and division panics, contract failures, exit codes, exact `Decimal` math, value printing.
- The backend gets a reference: compiled code must produce the same output as the interpreter on every conformance test.
- Not yet supported in the interpreter: agents, capabilities, `ask`, and `unsafe`. They come with Phase 2 and the native backend.
- Two implementations must stay in sync. The shared conformance suite keeps them honest.

## Alternatives considered

- **Cranelift first:** the fastest code soonest, but weeks before the first program runs, and blocked on outside crates in some environments.
- **Compile to C or JavaScript first:** quick, but adds a toolchain dependency and a different set of run-time bugs.
