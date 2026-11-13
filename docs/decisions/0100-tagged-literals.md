# 0100. Tagged literals, checked by their package

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

A package that speaks another language, such as SQL for PostgreSQL and SQLite, can check its text only when the program runs: a misspelled keyword reaches production. A maintainer asked (2026-11-12) for queries checked as far as possible when the program is compiled, with their syntax highlighted, and chose tagged literals from the options put to them; the declaration form was chosen on 2026-11-13. The PostgreSQL package's session raised the points the details below settle.

## Decision

- A package declares `pub literal name: T check f language "x"`: `T` a record whose first field holds the text, `f(text) -> [LiteralProblem]` a pure function, `language` optional (spec 10.15).
- The compiler runs `f` on each literal through the interpreter and reports each `LiteralProblem` at its column; a checker that panics or runs past a second is a compile error naming the tag.
- The text is raw: no escapes, no interpolation; `"""…"""` for lines.
- Outside the package, a `T` comes only from a literal; a function making one from run-time text is `audited fn`, listed by `kavach audit`.
- Generated tests make no `T` from made-up text; a function taking one is tested through its examples and `test` blocks.

## Consequences

- SQL mistakes become compile errors, and no query is built from user input by accident.
- `literal` and `audited` are words only where an item starts (`literal sql: …`, `audited fn`), not keywords, so names already called `literal` keep working.
- Each checker runs at every compile; one second is its limit.

## Alternatives considered

- **The tag on the type** (`type Query literal sql check …`): one item fewer, a long type line.
- **Literal-only parameters** (`fn sql(text: literal String)`), with `require` run at compile time: no highlighting, and a bigger compile-time engine.
- **Checking only when the program runs:** no compile errors.
