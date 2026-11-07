# 14.8 std.regex

Regular expressions, with two engines:

| Function | Gives |
|---|---|
| `regex.new(pattern) -> Regex ! RegexError` | A pattern for the **linear-time** engine: matching takes time proportional to the text, whatever the pattern and the text. Safe for patterns and text from users |
| `regex.advanced(pattern) -> Regex ! RegexError` | Also allows lookahead and lookbehind (`(?=…)`, `(?<!…)`) and backreferences (`\1`). These can take very long on some text, so matching gives up after a fixed amount of work |

```kavach
use std.regex

const email = try regex.new(r"^[\w.+-]+@[\w-]+\.[\w.]+$")
print(email.is_match("asha@example.com"))      // true

const date = try regex.new(r"(?<year>\d{4})-(?<month>\d{2})")
const parts = date.named_captures("due 2026-10") ?? [:]
print(parts["year"] ?? "")                      // 2026
```

Write patterns as raw strings (`r"…"`, 1.10) so `\d` needs no escaping. The syntax is the common one: classes (`\d`, `\w`, `\s`, `[a-z]`), repetition (`*`, `+`, `?`, `{2,5}`, lazy `*?`), groups, named groups `(?<name>…)`, anchors `^ $ `, flags such as `(?i)` for ignoring case, and Unicode.

Methods of `Regex`:

| Method | Gives |
|---|---|
| `is_match(text)` | Whether the pattern matches anywhere in `text` |
| `find(text)` | The first match, or `none` |
| `find_all(text)` | Every match that doesn't overlap the one before |
| `captures(text)` | For the first match: the whole match then each group, `none` for a group that didn't take part; or `none` without a match |
| `named_captures(text)` | For the first match: each named group that took part, by name; or `none` |
| `replace(text, with)`, `replace_all(text, with)` | The text with the first or every match replaced; in `with`, `$1` and `${name}` insert a group (write `with` as a raw string, `r"${name}"`, so `{…}` is not interpolation) |
| `split(text)` | The parts of `text` between matches |

A `Regex` prints as `Regex("pattern")`. Two are equal when their patterns are.

If an advanced pattern gives up on some text, that is a panic (5.6) saying so: for text from outside, prefer `regex.new`.
