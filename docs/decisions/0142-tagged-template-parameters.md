# 0142. Parameters in a tagged template

- **Status:** Accepted (points marked "for a maintainer's review" are open to change)
- **Date:** 2026-11-20

## Context

[Decision 0131](0131-plain-strings-and-backtick-templates.md) (point 4) said a tag before a template, `` sql`… where owner = {owner}` ``, takes each `{…}` as a parameter the tag writes its own way, never pasted into the text. It left open how a tag says how it writes a parameter (`$1`, `?`), and how the literal's value holds the values (spec 10.15's open question). Until now, a tag before a backtick was a compile error. This record extends [decision 0100](0100-tagged-literals.md), whose literals were raw text only.

## Decision

A maintainer chose (2026-11-20):

1. **Both forms stay.** `sql"…"` is raw text with no parameters, as before. `` sql`…` `` reads like every other template (spec 1.7): escapes, `\{` for a brace, `` \` `` for a backtick, and lines when its text starts on the line after the backtick. Each `{…}` in it is a parameter.
2. **The tag names the function that writes a placeholder:** `literal sql: Query check check_sql param pg_param`, with `fn pg_param(n: Int) -> String`. The n-th `{…}` (from 1) is replaced in the text by `pg_param(n)`: `$1`, `$2` for PostgreSQL, `?` for SQLite. The checker reads the text with the placeholders in, so it checks real SQL, and its problems are shown at their place in the source (inside a placeholder: at its `{…}`). The function must be pure, with no `!` and no `uses`, like the checker.
3. **The values are held as one type**, in a field `params: [P]` of the tag's record, in order. A tag with no `param` refuses `{…}`; a literal with no `{…}` has `params: []`.

Chosen in this change, **for a maintainer's review**:

- **How the tag names `P`:** by the type of its record's `params` field. `T` must have `params: [P]` when the tag has `param`. Nothing new to write on the tag's line, and the record already says what it holds.
- **How a value converts to `P`:** it converts when it is a `P`; when `P` is `Json`, as an item of a `Json` list converts (`json.of`, spec 2.12, decision 0132), so every value does; and when `P` is a union with exactly one variant whose only field has the value's type, as that variant. Anything else is a compile error at its `{…}`. Kavach has no other conversion into a union (a variant is always written with its union's name, decision 0070); the one place values become a union's variants without being named is a `Json` literal, so `Json` follows that rule and other unions get the narrowest rule that still lets a package define its own `Param`. Both packages (`pg`, `sqlite`) take `[Json]` today, so they use `params: [Json]`.
- **When the placeholders are written:** the checker's text is made while the program is compiled. The value's `text` is made the same way when the literal is evaluated, by calling the same pure function, so it is the same text; neither back end has to carry compile-time results to the other.

## Consequences

- `` sql`… where owner = {owner}` `` reads like the SQL it runs, and the value still never reaches the text: injection stays impossible.
- A tag's checker sees `$1` where the value goes, as the database will.
- A `{…}` written with a tag that takes no parameters is an error, not a silent paste.
- A package's query functions read `params` from the `Query`, so a query written as a template needs no separate list of values.

## Alternatives considered

- **A field per parameter, named after the expression:** a record type per literal; drivers want one list.
- **The tag names `P` on its line** (`param pg_param as Json`): one more thing to keep in step with the record's field.
- **A conversion function on the tag** (`param pg_param from to_param`): most general, but one more function to write and check for each tag; the `Json` rule covers both packages.
- **Writing the placeholder text once, at compile time, into the program:** the same text, but the interpreter's compile-time results would have to reach native code generation.
