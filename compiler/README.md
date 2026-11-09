# compiler/

Turns `.kav` source code into native machine code or WASM.

Written in **Rust** ([decision 0004](../docs/decisions/0004-compiler-written-in-rust.md)). Each subfolder becomes one Rust crate named `kavach-<folder>`.

## Pipeline (in order)

| Folder | Job |
|---|---|
| [lexer/](lexer/README.md) | Source text → tokens. **Done:** 17 tests, in sync with spec chapter 1 |
| [parser/](parser/README.md) | Tokens → syntax tree. **Done:** chapters 1–8 syntax |
| [ast/](ast/README.md) | Syntax tree data types, shared by every later stage. **Done** |
| [resolver/](resolver/README.md) | Connects names to definitions; handles `use` and modules. **Done** for single files |
| [checker/](checker/README.md) | Types, capabilities, contracts, secrets, ownership. **Done** for chapters 2–7 |
| [interp/](interp/README.md) | Development interpreter: runs a checked program directly ([decision 0014](../docs/decisions/0014-development-interpreter.md)). **Done** for chapters 1–7 |
| [kir/](kir/README.md) | Kavach IR: typed program form for the code generators. **Most of chapters 1–7**; not yet errors, lambdas, maps |
| [backend/cranelift/](backend/cranelift/README.md) | Fast compiles for development. **Started:** `kavach run --native` and `kavach build`, matching the interpreter on every supported test |
| `backend/llvm/` | Fully optimized release builds and WASM |
| [driver/](driver/README.md) | Runs the whole pipeline and renders messages; used by the CLI, the interpreter and (later) the language server. **Done** |

See [architecture overview](../docs/architecture/overview.md) for how the stages fit together.

**Status:** Phase 1 has started ([roadmap](../docs/vision/roadmap.md)). The front end is done: lexer, parser, resolver and type checker. Every compile-error conformance test for chapters 1–7 is caught. Programs run with the development interpreter (`kavach run`); all conformance tests for chapters 1–7 pass. Native code runs 89 of the conformance tests and examples, in memory and as built executables, matching the interpreter exactly with no memory leaks: numbers (including `Decimal`), text, lists, maps, records, unions, optionals, `Shared`, methods, contracts, errors, closures, generics and the standard library of spec 14, with values moved at their last use. Only agents (Phase 2) remain. All crates are members of the workspace in the root `Cargo.toml`.
