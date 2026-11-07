# 0037. Destructuring as JavaScript does, with Kavach's types

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

Mithlesh asked for `const [two, three] = [2, 3]` (2026-11-02), and to follow JavaScript's rules. Kavach already takes tuples apart (`const (code, message) = pair`, spec 2.3). Lists and records it couldn't.

JavaScript's rules: an array pattern takes items by position; a missing item is `undefined` and extra items are ignored; a name may have a default (`[a = 1]`), used when the item is missing; `...rest` gets the items left; holes skip items (`[, b]`). An object pattern takes properties by name, may rename (`{ age: years }`), and may have defaults. A list's length isn't known when compiling, so in Kavach a missing item has to show in the type.

## Decision

**`const` and `let` take lists apart by position and records by field name, as JavaScript does; what may be missing is optional (spec 1.12).**

1. `const [a, b] = xs` gives `a` and `b` of type `T?`: `none` for a missing item, as JavaScript gives `undefined`. Extra items are ignored.
2. A default makes the name a `T`: `const [a = 0] = xs`.
3. `...rest`, last, gets the items left, as a `[T]`; `_` skips an item, Kavach's spelling of JavaScript's hole.
4. When the value is a list literal with an item for every name, nothing can be missing, so the names get the items' own types: `const [two, three] = [2, 3]` gives two `Int`s.
5. `const { name, age: years } = user` takes a record's fields; a field the record doesn't have is a compile error, since a record's fields are known. A default fills a `T?` field that is `none`.
6. A map is taken apart by key, the pattern written like a map literal (Mithlesh, 2026-11-02): `["asha": a]`, or a bare name for the text key spelled like it, `[port = 8080]`, as JavaScript's `{ port = 8080 }`. A key may be missing, so a name is `V?`, or `V` with a default. Whether `[a, b]` reads positions or keys follows the value's type, a list or a map.
7. Patterns nest (Mithlesh, 2026-11-02). When the part a nested pattern takes apart may be missing, every name inside it is `none` when it is, so their types are optional unless they have defaults. JavaScript throws a `TypeError` there; Kavach keeps its rule that destructuring never fails.
8. Not in this step: list patterns in `match`, where a pattern would test the length, a different meaning.

## Consequences

- Code reads as people write it in JavaScript and Python, without hidden `undefined`s: the compiler makes a possibly missing item a `T?`, so it must be handled before use.
- Destructuring never fails at run time, as in JavaScript.
- `...` is a new token; `..` stays the range operator.

## Alternatives considered

- **Panic when the length differs** (as an out-of-range index does): plain types, but a crash for a short list, which JavaScript never does.
- **Only in `if let` and `match`:** safe, but not what was asked for, and wordier for the common case.
- **`..rest` instead of `...rest`:** one token less, but `..` already means a range, and JavaScript writes `...`.
