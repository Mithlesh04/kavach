# 0091. Workspaces, and packages in a folder of a repository

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Rath, a web framework written in Kavach, is a router, sessions, templates and more: packages meant to be used apart, kept in one repository and changed together. It asked (2026-11-12) for git dependencies from a folder of a repository, and for workspaces.

A maintainer chose (2026-11-12) the first of the options put to them: a `[workspace]` table in the root `kavach.toml`, as Cargo and pnpm have.

## Decision

- **`dir`** on a git dependency (spec 10.12) names the package's folder in the repository. It must stay inside the repository: no leading `/` or `-`, no `..` part.
- **`[workspace] members = ["packages/*"]`** (spec 10.14) in a root `kavach.toml` makes the listed folders one workspace. `*` stands for any one folder name.
- **One `kavach.lock` and one `kavach_modules/`**, at the root, for every member, so all members use the same commit of each dependency and fetch it once.
- **`kavach check`, `test` and `build`** at the root, with no file, run on every member in the order listed, each member's lines headed by its folder; the command fails when any member does.

## Consequences

- A framework and its plugins live in one repository, with one lock, and are tested together.
- Users fetch one package of such a repository with `dir`, without the rest being compiled into their program.
- A member is still a whole package: its own `kavach.toml`, settings and `src/`.

## Alternatives considered

- **Only `dir` for now:** fetching works, but working on several packages together would need a lock and a fetch per package.
