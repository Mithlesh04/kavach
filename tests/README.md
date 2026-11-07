# tests/

Language-level tests. Unit tests for compiler code live next to that code instead.

| Folder | Contents |
|---|---|
| [conformance/](conformance/README.md) | Small `.kav` programs that check one spec rule each. Subfolders mirror spec chapters: `01-basics/`, `02-types/`, … |
| `integration/` | Larger programs that exercise many features together (CLI, packages, agents) |

## Conformance tests

Tests cover spec chapters 1–7, 8.1, 9.1, 10.1–10.3, 11, 12 and 14, one rule per file. A folder may have its own `kavach.toml`, which the runner reads. The test format and the list of chapters are in [conformance/README.md](conformance/README.md).

Conformance tests check the **language rules**. Programs written in Kavach need far fewer hand-written tests, because `kavach test` generates them from types and contracts ([spec chapter 11](../docs/spec/11-testing.md)).

Why this matters: conformance tests are written against the **spec**, not the compiler. Any future Kavach implementation can prove it is correct by passing them. That is part of how the language outlives any single implementation.
