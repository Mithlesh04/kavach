# 0015. Token efficiency for AI is a design goal

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

AI assistants write a growing share of all code, and every token they read or write costs money and time. Watching them work, most tokens go to **finding and reading code**: text searches, then whole files, to change a few lines. Then more go to repeating unchanged code to place an edit, writing tests, and retrying after unclear errors. No mainstream language is designed to reduce this.

Kavach is unusually well placed. Its signatures carry types, errors, capabilities, contracts and examples (spec 3.5), so a signature says how to use a function without its body. Its resolver knows every reference exactly. Its tests come from contracts (chapter 11).

## Decision

Make "at least 90% fewer tokens for the same task" a design goal of the language and its tools (spec chapter 13):

- **Query instead of search:** `kavach query` answers questions about code exactly (callers, users of a capability, types).
- **Outline and context instead of reading:** `kavach outline` shows signatures without bodies; `kavach context` returns exactly what a change needs, within a token budget.
- **Edit by name:** `kavach edit` and `rename` change declarations by path, checked before saving.
- **Errors with fixes:** compact, machine-readable, with fixes the tools can apply.
- **One guide, one server:** `llms.txt` and an MCP server, so any assistant can use all of this.
- **Measure it:** a published token benchmark (`benchmarks/tokens/`).

## Consequences

- A strong, measurable reason for teams using AI to adopt Kavach, beyond safety.
- New features are judged partly by their token cost: if a feature makes code or tooling output longer, that needs a reason.
- More tooling to build and keep correct. Most of it reuses the compiler's own resolver and checker, so it stays in sync.
- The 90% target may not hold for every task. The benchmark reports honestly where it doesn't.

## Alternatives considered

- **Leave it to AI tools (better search, embeddings):** they help, but they guess. Only the compiler knows exactly what a name refers to.
- **Generic code-intelligence protocols only (LSP):** designed for editors and people, not token budgets; responses are verbose and need many round trips.
