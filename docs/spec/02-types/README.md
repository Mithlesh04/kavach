# 2. Types

Kavach is statically typed: every value has a type known at compile time. Types are inferred almost everywhere, so you rarely write them (see 2.10).

Each section is a file of its own: new sections go at the end, with the next number.

| Section | File |
|---|---|
| 2.1 Built-in types | [01-built-in-types.md](01-built-in-types.md) |
| 2.2 Collections | [02-collections.md](02-collections.md) |
| 2.3 Tuples | [03-tuples.md](03-tuples.md) |
| 2.4 Records | [04-records.md](04-records.md) |
| 2.5 Methods | [05-methods.md](05-methods.md) |
| 2.6 Unions | [06-unions.md](06-unions.md) |
| 2.7 Optionals | [07-optionals.md](07-optionals.md) |
| 2.8 Interfaces | [08-interfaces.md](08-interfaces.md) |
| 2.9 Generics | [09-generics.md](09-generics.md) |
| 2.10 Where types must be written | [10-where-types-must-be-written.md](10-where-types-must-be-written.md) |
| 2.11 Secret fields | [11-secret-fields.md](11-secret-fields.md) |
| 2.12 Json | [12-json.md](12-json.md) |
| 2.13 Bytes | [13-bytes.md](13-bytes.md) |
| 2.14 Anonymous records | [14-anonymous-records.md](14-anonymous-records.md) |
| 2.15 Interface values | [15-interface-values.md](15-interface-values.md) |
| 2.16 Operators for your own types | [16-operators.md](16-operators.md) |
| 2.17 Matrices: now tensors of two dimensions (14.33) | [17-matrices.md](17-matrices.md) |
| 2.18 Text views and builders | [18-text-views.md](18-text-views.md) |
| 2.19 Indexing and slicing | [19-indexing-and-slicing.md](19-indexing-and-slicing.md) |
| 2.20 Frozen values | [20-frozen-values.md](20-frozen-values.md) |

## Open questions

- **`a ?? b` with an optional `b`** gives a `T?`, the first that holds a value (2.7), as in Swift, C# and Kotlin. Built 2026-11-22, for a maintainer's review.
- **Changing a `Json` in place.** `doc["name"] = "Ravi"` on a `let` holding an object would be convenient; today you change the dict from `object()` and build a new `Json`.
- **Decoding unions:** the `{"Variant": {…}}` form only, or also a tag field (`"type": "Circle"`) chosen on the union?
- **`bits` and range numbers, for a maintainer's review** (2.1, built 2026-11-21): a conversion of a number into any whole-number type fails with a `SizeError` under `try` or `catch`, `Int8` … `UInt64` included, with the panic's text (`300 doesn't fit in Int8`); a literal on the right of an operator or a comparison on a range (`p - 1`, `p += 5`, `p > 0`) need only be an `Int`; the bit operators are compile errors on a range; `Int<range: 0..10>` is the type `Int<range: 0..=9>` and is shown so; `UInt` alone is an error naming `UInt<bits: 32>`; the JSON Schema of a `bits` or range number is a plain `{"type": "integer"}`, without `minimum` and `maximum`; widths over 64 bits wait (`Int<bits: 128>` names `BigInt`).
- **`BigInt` and bits:** should `& | ^ << >> ~` work on `BigInt`, as on the other whole numbers? Not yet.
- **`BigInt` back to `Int`:** comes with the conversions planned for every built-in type (`Int(b)`, failing when it doesn't fit).
- **A number type's range as constants:** `Int.max` and the like are a compile error today, since `max` is already a method (`a.max(b)`); names such as `Int.largest` and `Int.smallest`, or none, with `checked_add` and the others (5.6) for arithmetic that may not fit.

Decisions are recorded in [0013](../../decisions/0013-core-language-decisions.md) and, for `Json`, [0019](../../decisions/0019-json-values.md).
