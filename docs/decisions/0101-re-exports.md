# 0101. Re-exports with `pub use`

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

A package shows its public types through its root module: a user of the PostgreSQL package writes `pg.Client`. So every public type had to be declared in the package's root file, `lib.kav`, however large the package grew. Decision record 0098's packages, written in Kavach in repositories of their own, hit this first: `pg` wanted `Client` in `client.kav` and `Pool` in `pool.kav`. An `extend` in another file can already hold a type's methods (spec 10.1), but not the type itself. A maintainer chose re-exports (2026-11-13).

## Decision

- **`pub use client.{Client, connect}`** in a module makes those names part of that module too: `pg.Client` and `use pg.{Client}` from another module reach the declaration in `client.kav` (spec 10.2).
- The name is the same declaration, not a copy: a `Client` from `pg.Client` and one from `pg.client.Client` are one type, and a `match` on a re-exported union covers the same variants.
- Only names are re-exported, each `pub` where it is declared. A whole module (`pub use orders`) is not: name what the module shows.
- A re-export may itself be re-exported, as each module shows what it names.

## Consequences

- A package keeps one public type per file and shows them all from its root.
- A module's public names are its declarations and its `pub use` names; a reader finds the declaration by following the `use`.

## Alternatives considered

- **No re-exports:** public types stay in the root file, and only methods move out with `extend`. Simple, but a large package's root file grows without end.
- **Whole-module re-exports (`pub use orders.*`):** fewer lines, but a module's public names would change whenever another file's do, unseen.
