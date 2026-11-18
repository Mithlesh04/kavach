# 0129. Maps written with braces

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

A map literal was written with brackets, `["asha": 31]`, and an empty map `[:]`, as in Swift. Most developers know maps from JSON, JavaScript and Python, where they are written with braces. A maintainer chose braces on 2026-11-17 (to-do item 18), the one way, with keys as JavaScript writes them.

## Decision

```kavach
const ages = {"asha": 31, "ravi": 27}
const empty: [String: Int] = {:}
const user = {name: "Asha", city: "Pune"}      // the text keys "name" and "city"
const key = "role"
const roles = {[key]: "admin"}                 // the value of `key`
const counts = {1: "one", 2: "two"}            // a number key
```

1. A map is written `{key: value, …}`, and an empty one `{:}`. `["a": 1]` and `[:]` are compile errors that say to write braces. Lists stay `[1, 2]`, and a map's type stays `[K: V]`.
2. A key written as a plain name is that text: `{name: "Asha"}` has the key `"name"`. A key in brackets is the value of what is inside: `{[key]: 1}`. Other keys are written as values: text in quotes (`{"first name": 1}`), numbers, `true` and `false`.
3. `{` after `if`, `else`, `fn`, `for`, `while`, `match`, `arena`, `unsafe`, `=>` and the like still starts a block. Elsewhere, where a value is expected, `{` starts a map when `:` follows it (`{:}`), when a key and `:` follow (`{"a":`, `{1:`, `{name:` but not a loop's label, `{rows: for`), or when `[…]:` follows.
4. `{}` stays an empty block; an empty map is `{:}`.
5. Map patterns (1.12) keep their brackets, `const [host: h] = config`: braces in a pattern are a record's (`{ name, age }`).
6. A map prints as it is written, `{"a": 1}` and `{:}`, as collections print as they are written (1.11); a maintainer kept this on 2026-11-18.

## Consequences

- Every map literal in the repository, the docs site and the packages changes from brackets to braces.
- `kavach fmt` keeps what is written; printed maps change from `["a": 1]` to `{"a": 1}`, and tests' expected output with them.
- The parser decides between a block and a map by looking at most past one key.

## Alternatives considered

- **Both forms:** two ways to write the same thing, which Kavach avoids.
- **Keeping `["a": 1]`:** familiar from Swift, but fewer developers know it than braces.
- **Keys as values only** (`{name: 1}` reading the variable `name`, as Python): the maintainer chose JavaScript's rule, with `[key]` for a variable.
