# 0131. Plain strings in quotes, templates in backticks

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

Every string filled in `{…}` and read escapes, so text that holds braces or backslashes (paths, regular expressions, JSON, templates for other tools) needed `\{`, `\\` or the raw form `r"…"`. A maintainer wanted plain text to be plain, and asked on 2026-11-18 for a way to tell the two apart. The options were keeping today's strings, single-quoted templates, Python's `f"…"`, and backticks as JavaScript; the maintainer chose backticks, with nothing processed inside quotes.

## Decision

```kavach
const path = "C:\temp\{name}"                  // exactly these characters
const greeting = `Hello, {name}! {items.len()} items`
const lines = `first\nsecond\t{total}`         // escapes work in templates
const tick = `run \`kavach build\` first`      // a backtick inside a template
const query = sql`select id from orders where owner = {owner}`
const letter = 'x'                             // a Char, as before
```

1. **`"…"` is exactly what is written**: no escapes and no interpolation. `"C:\path\{x}"` is exactly those characters. A `"` inside goes in `"""…"""`, or in a template. `r"…"` goes: a quoted string is raw already.
2. **Backticks are templates**, as in JavaScript: `` `…{expr}…` `` fills in any expression, and reads the escapes `\n`, `\r`, `\t`, `\\`, `` \` ``, `\{` and `\u{…}`. A template may span lines; its common indentation is removed as `"""…"""` does.
3. **`'x'` stays a `Char`**, with its escapes.
4. **Tagged literals follow the same rule.** `sql"…"` is plain text, checked as today. In `` sql`…{owner}…` `` each `{…}` is a parameter that the tag writes its own way (`$1`, `?`), never pasted into the text, so values can't change what the text means. A tag that takes no parameters refuses `{…}`. This part waits for 10.15's open question; until then a tag before a template is a compile error that says to write the text in quotes.
5. **A plain string that looks like it wanted a template** gets a warning that says to use backticks: `"Hello, {name}"` when `name` is a name in scope there, and `"a\nb"` when every backslash is `\n`, `\t` or `\r` and the text isn't a Windows path (`C:\new`). A route pattern such as `"/users/{id}"`, with no `id` in scope, and a regular expression get none.
6. **No shims.** Every string in the repository, the docs site and the packages that fills something in or uses an escape becomes a template.

## Consequences

- Text with braces and backslashes, the common case in paths, patterns and JSON, needs nothing special.
- Every string with `{…}` or an escape changes, by a converter, across the repository and the packages.
- How a tag declares its parameters' form and receives their values is specified in 10.15 before the tagged part is built.

## Alternatives considered

- **Keeping today's strings:** one kind of string, but plain text needs escapes or `r"…"`.
- **`'…'` templates:** short, but `'x'` is a `Char`, which would have to move.
- **Python's `f"…"`:** familiar, but a letter before every template.
