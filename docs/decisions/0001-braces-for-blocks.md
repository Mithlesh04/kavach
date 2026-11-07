# 0001. Braces for blocks, no semicolons

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Blocks can be marked with braces `{ }` (C#, Java, Rust, Go), indentation (Python) or `end` keywords (Ruby, Lua). Much Kavach code will be generated, pasted and edited by AI tools, where whitespace often gets damaged.

## Decision

Blocks use braces. Statements end at the end of a line; there are no semicolons.

## Consequences

- Code survives copy-paste, chat interfaces and AI generation without changing meaning.
- Familiar to the large C#, Java, JS/TS, Go and Rust audience.
- `kavach fmt` handles indentation, so it is still consistent.
- Newline rules (spec 1.1) must be precise so line continuation is never surprising.

## Alternatives considered

- **Indentation:** clean to read, but one lost space changes the program's meaning, which is risky with generated code.
- **`end` keywords:** robust, but wordy and less familiar to most developers.
