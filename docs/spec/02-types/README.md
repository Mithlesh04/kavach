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

## Open questions

- **Changing a `Json` in place.** `doc["name"] = "Ravi"` on a `let` holding an object would be convenient; today you change the map from `object()` and build a new `Json`.
- **`Bytes` in JSON:** base64 text, or a compile error as now?
- **Decoding unions:** the `{"Variant": {…}}` form only, or also a tag field (`"type": "Circle"`) chosen on the union?
- **`BigInt` and bits:** should `& | ^ << >> ~` work on `BigInt`, as on the other whole numbers? Not yet.
- **`BigInt` back to `Int`:** comes with the conversions planned for every built-in type (`Int(b)`, failing when it doesn't fit).

Decisions are recorded in [0013](../../decisions/0013-core-language-decisions.md) and, for `Json`, [0019](../../decisions/0019-json-values.md).
