# 0133. A short license header in every source file

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

[Decision 0011](0011-license-apache-or-mit.md) licenses the whole repository under Apache-2.0 OR MIT. The license texts sit at the repository root. A single file says nothing about its license once it is copied out of the repository: into another project, a package, or a vendored folder.

A maintainer asked on 2026-11-19 that every source file carry a short header. It must not cost much: every file is held to 500 lines ([decision 0009](0009-size-limits-and-enforced-architecture.md)), and nobody wants to edit hundreds of files every January.

## Decision

Every source file in `compiler/`, `runtime/`, `tools/`, `stdlib/`, `interop/`, `scripts/` and `editors/` starts with three lines:

```
// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach
```

- **Which files:** `.rs`, `.kav`, `.sh`, `.ps1`, `.js`, `.mjs` and `.ts` files, and the git hooks in `scripts/hooks/`. Rust test files in those folders get it too.
- **Comment marker:** shell scripts, PowerShell scripts and hooks use `#` instead of `//`.
- **Place:** the first lines of the file. A shebang (`#!`) stays first, with the header right after it. In Rust, the header comes before the `//!` lines.
- **Holder:** the same text as in `LICENSE-MIT`.
- **Year:** the year the file was created. It is never updated.
- **Left out:** `tests/`, `examples/`, `website/` and `benchmarks/`; generated files (`*.lock`, `tests/found/`, files marked `@generated` or "do not edit"); vendored code; Markdown, JSON and TOML; and test fixtures whose text or line numbers a test compares.
- **Enforced:** `scripts/check-repo.sh` and its `.ps1` twin fail on a file in scope whose first 5 lines lack the copyright line or the SPDX line. The scope and the list of what is left out live there, in one place. `scripts/add-license-headers.sh` (and `.ps1`) adds the header, with the current year, to every file the check reports.

### Why SPDX

`SPDX-License-Identifier` is one short line with a standard meaning. License scanners, package tools and `cargo-deny` read it, so no one has to guess a file's license from its text. It also replaces a long banner.

### Why the links

The site and repository links tell a reader of a copied file where it came from, where the docs are, and where to report a problem.

### Why the creation year, never updated

A copyright year records when the work was first published. Updating it every year adds nothing legally, and it would change every file every January: a large, noisy commit with no real change in it. A file keeps the year it was created. A new file gets the year it is added.

### Why tests, examples and samples are left out

- `tests/conformance` files start with lines the test runner reads (`// Rule:`, `// Spec:`, `// expect`), and expected output often names line numbers. A header would break both.
- Examples, website samples and benchmarks are what people read and copy to learn Kavach. Three extra lines on each short program get in the way, and they are covered by the repository license anyway.
- Test fixtures elsewhere whose text a test compares, such as the VS Code grammar tests, whose first line must be `SYNTAX TEST`.

## Consequences

- Each source file states its license and origin, even when copied alone.
- Every source file grows by four lines (the header and a blank line). They count toward the 500-line limit.
- New files need the header. The repo check catches a missing one, and `add-license-headers` adds it.
- When a new kind of fixture must not change, it goes on the skip list in both check scripts.

## Alternatives considered

- **A fixed first year (2026) in every file:** simple, but wrong for files created in later years.
- **A year range, updated every year (`2026-2027`):** changes every file every January, for no legal gain.
- **No year at all (LLVM's choice):** short, and also valid. But the MIT license text names a year, and a creation year is cheap when it never changes.
- **A long banner with the license summary:** clear, but costs 10 to 15 lines in every file against the 500-line limit, and the SPDX line says the same thing.
- **No header (license only at the root):** the current state; says nothing about a file copied out of the repository.
