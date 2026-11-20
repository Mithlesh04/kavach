# 0143. Text over lines only in backticks

- **Status:** Accepted (points marked "for a maintainer's review" are open to change)
- **Date:** 2026-11-20

## Context

[Decision 0131](0131-plain-strings-and-backtick-templates.md) gave Kavach two ways to write text over lines: `"""…"""`, exactly as written, and a template in backticks whose text starts on the line after the backtick. Both drop the line breaks next to their delimiters and remove the common indentation. Two forms for one job is one too many, and `"""…"""` was also the only way to put a `"` in plain text. A maintainer asked on 2026-11-20 whether backticks could be the one form, then chose it.

This record replaces the parts of decision 0131 about `"""`: point 1 ("A `"` inside goes in `"""…"""`") and the comparison with `"""` in point 2. It also replaces "three quotes let a literal span lines" from [decision 0100](0100-tagged-literals.md) and spec 10.15. The rest of decision 0131 stands.

## Decision

```kavach
const letter = `
    Dear {name},
      your order {id} is on its way.
    `
const body = `
    \{"name": "{name}", "path": "C:\\temp"}
    `
const query = sql`
    select id from orders
    where owner = {owner}
    `
```

1. **Text over lines is written only in backticks.** A template whose text starts on the line after its backtick spans lines, as before (spec 1.7): the line break after the opening backtick and the one before a closing backtick on its own line are left out, and the common indentation is removed. It fills in `{…}` and reads escapes, as every template does.
2. **`"""…"""` is a compile error** that shows the backtick form: write the text in backticks, with `\{` for a brace, `` \` `` for a backtick and `\\` for a backslash. The compiler still reads to the closing `"""`, so it reports one error and checks the rest of the file.
3. **`sql"""…"""` is a compile error too**, that shows `` sql`…` ``. A tagged template's `{…}` are parameters (decision 0142), so `\{` writes a brace there as well.
4. **Text in quotes stays as it is:** one line, exactly as written. A `"` inside text goes in a template.
5. **No shims.** Every `"""` in the repository becomes a template, with `\`, `` ` `` and `{` escaped, so each text means exactly what it did.

Chosen in this change, **for a maintainer's review**:

- **`r"""…"""` gets the one `"""` error**, not also the "drop the `r`" error, so a mistake is reported once.
- **Tabs in the indentation stay an open question** (spec 1.7). Today a tab and a space each count as one character of indentation.

## Consequences

- One form for text over lines, and one set of rules: braces and backslashes in text over lines (JSON, paths, regular expressions) are escaped, as in any template.
- JSON over lines writes `\{` for each opening brace. A closing `}` needs nothing.
- Editors colour `"""` as a mistake.
- The docs site's samples that use `"""` change with it.

## Alternatives considered

- **Keeping both forms:** plain text over lines needs no escapes, but two forms do one job, and a reader has to know both.
- **Plain text over lines in quotes, `"…"` spanning lines:** no escapes, but a missing `"` would swallow the rest of the file instead of stopping at the line's end.
