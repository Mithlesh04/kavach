# 0083. Dependencies by path and git, before a registry

- **Status:** Accepted
- **Date:** 2026-11-11

## Context

A maintainer asked (2026-11-11) whether people can write packages and frameworks in Kavach. They could not: `kavach.toml` accepted `[dependencies]` but nothing read it, so one Kavach project could not use another. The roadmap had the package manager and registry in Phase 3. The maintainer asked for the call to be made and the work done fast.

## Decision

- **Path and git first, no registry yet** (spec 10.12). A dependency is `{ path = "…" }` or `{ git = "…" }` with at most one of `tag`, `branch` and `rev`. A version number is an error that says there is no registry yet. Git is run as the `git` command, so no new outside dependency.
- **A dependency is a library package,** and its name in `[dependencies]` is its module name: `src/lib.kav` is the module `geometry`, `src/shapes.kav` is `geometry.shapes`. Every file carries its package (`Source::package`, `Module::package`); inside a package, `use shapes` looks for the package's own module first, so a package means the same in every project.
- **`kavach.lock` records each git dependency's exact commit;** fetched repositories go in `.kavach/deps/<name>-<commit>`. Only `kavach add`, `kavach update` and a build that finds something missing use the network; compiling reads the lock and the folders.
- **`unsafe` must be allowed per dependency** (`unsafe = true` in its entry), checked per package by the type checker. Warnings in a dependency's code are not shown.
- One package per name across all dependencies; two different ones with the same name are an error.

## Consequences

- People can now write and share Kavach libraries and frameworks as git repositories, and pin them.
- Still to come (spec 10 open questions): a registry with version numbers and their selection, package signing, and whether a dependency must declare the capabilities it uses.
- The language still changes without migrations until edition 1 is frozen ([roadmap](../vision/roadmap.md), Phase 3), so packages written now may need updating.

## Alternatives considered

- **Waiting for the registry (Phase 3):** nobody could share code until then.
- **A dependency's modules under their plain names:** two packages with a `utils` module would clash.
