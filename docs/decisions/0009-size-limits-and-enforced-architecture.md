# 0009. 500-line file limit and enforced architecture

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Large codebases decay in predictable ways: files grow to thousands of lines, modules depend on things they shouldn't, and shared libraries break their users by accident. Style guides and code review try to stop this, but they depend on people remembering, and they don't scale across many teams.

Long files also hurt AI-assisted development. A model, like a person, works better with a small file that does one thing.

## Decision

- **This repository:** no file we write may exceed 500 lines, counting every line. Enforced by `scripts/check-repo.sh` / `.ps1`. Rules for contributors and AI agents are in `AGENTS.md`.
- **The language:** Kavach enforces the same default for its users: `[limits] file_lines = 500` (error) and `fn_lines = 80` (warning), configurable in `kavach.toml`, with per-file exceptions listed centrally.
- **At scale:** workspaces, incremental rebuilds and tests, `[architecture]` dependency rules, no dependency cycles, `kavach api check` for breaking changes, and `deprecated … fix …` with `kavach fix` migrations.

(Spec chapter 12.)

## Consequences

- Files stay small enough to read and review in one sitting, by people or AI.
- Architecture is written down in `kavach.toml` and checked by the compiler, so it can't drift silently.
- Breaking changes are caught before release, including the ones other languages miss (new capability, new error variant, stronger `require`).
- Some friction: a developer occasionally has to split a file they would have kept whole. The compiler suggests where to split, and exceptions are possible but visible.

## Alternatives considered

- **No limit, leave it to linters:** linters are optional and every team configures them differently. Defaults decide behaviour for most people.
- **Limit only code lines, not comments:** this would push people to delete or avoid useful comments.
- **A lower limit (200–300 lines):** produces many tiny files and too much jumping between them.
- **Architecture rules in a separate tool (like ArchUnit):** works, but few teams set it up. Built into the compiler, it's available to everyone at no cost.
