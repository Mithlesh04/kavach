# 2.18 Text views and builders

A `String` is a value that never changes (1.7), and a slice `s[a:b]` (2.19) gives a copy. Two built-in types make text work fast without giving that up: a `StrView` is part of a string with nothing copied, for parsing; a `TextBuilder` is text being made, changed in place, for building.

```kavach
const line = "  id=42, name=Asha  "
const fields = line.view().trim().split(", ")     // [StrView]: no text copied
for f in fields {
    const parts = f.split("=")
    print(`{parts[0]} is {parts[1]}`)              // id is 42, name is Asha
}

let out = TextBuilder()
for n in 1..=3 {
    out.push("item ")
    out.push(`{n}`)
    out.push_char('\n')
}
out.replace(0, 4, "ITEM")                          // the first "item" only
const text = out.to_string()                       // "ITEM 1\nitem 2\nitem 3\n"
```

**Views.** `s.view()` is all of `s`, and a slice of a view is a view: `s.view()[start:end]` is characters `start..end` of `s`, clamped as any slice is (2.19), with nothing copied. A view keeps its string alive and can't change it. `view(start, end)` is a compile error that names the slice (2.19).

| Method | Gives |
|---|---|
| `v.len()`, `v.is_empty()` | Its length in characters, and whether it has none |
| `v[i]` | Its character `i`, a `Char`, as for a `String` (2.19) |
| `v[start:end:step]` | A slice of it (2.19), as a view: of the same string, or, with a step other than 1, of a new string holding those characters |
| `v.trim()`, `v.trim_start()`, `v.trim_end()` | It without the spaces at its ends, as a view |
| `v.split(sep) -> [StrView]` | Its parts between each `sep`, as views |
| `v.lines() -> [StrView]` | Its lines, without their line ends, as views |
| `v.find(text) -> Int?` | Where `text` first starts in it, in characters |
| `v.starts_with(text)`, `v.ends_with(text)`, `v.contains(text)` | As their names say |
| `v.to_string() -> String` | Its text, copied: a string of its own |

- A view compares with `==` and `<` to another view or to a `String`, by its text; `` `{v}` `` and `print` show its text.
- `Int(v)`, `Float(v)` and the other conversions from text (2.1) take a view too, so a number is read without copying it out.
- A view is a value: it can be stored, put in a list and given to a task. It holds its whole string, so a small view of a big text keeps the big text in memory; `to_string()` lets it go.

**Builders.** `TextBuilder()` is empty, `TextBuilder(text)` starts with `text`, and `TextBuilder.capacity(n)` has room for `n` bytes before it grows. A builder changes in place, so it is a `let`.

| Method | Does |
|---|---|
| `b.push(text)`, `b.push_char(c)` | Adds a `String`, a `StrView` or a `Char` at its end |
| `b.insert(at, text)` | Puts `text` before character `at` |
| `b.replace(start, end, text)` | Puts `text` in place of characters `start..end` |
| `b.remove(start, end)` | Takes out characters `start..end` |
| `b.clear()` | Empties it, keeping its room |
| `b.len()`, `b.is_empty()` | Its length in characters, and whether it has none |
| `b.view() -> StrView` | Its text now, as a view, with nothing copied |
| `b.to_string() -> String` | Its text, copied |

- A position past the end is clamped, as in a slice (2.19); `start` after `end` is a panic (a contract).
- A copy of a builder (`let c = b`) is a builder of its own: changing one never changes the other (6.1).
- A view of a builder keeps the text it had: when the builder changes while a view of it is held, the builder copies its text first, once, as any value shared by two holders does (6.1).

## Open questions

- Positions in bytes as well as characters, for code that reads binary formats: `v.byte_len()`, `v.view_bytes(a, b)`.
- A `StrView` of a `Bytes` read as UTF-8, with no copy.
