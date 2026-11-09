# 0028. Anonymous records: Python's speed of writing, fully typed

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

Developers pick Python for how fast a shape can be written down, and TypeScript or C++ when they need the checking — and TypeScript won web companies by letting code start loose and harden gradually, without a rewrite. In Kavach today every shape needs a named `type` first, even one used once between two functions (Mithlesh, 2026-11-01). The checking side is already strong: full inference for locals, contracts, generated tests, capabilities. The writing side needs the shape-on-the-fly step.

## Decision

**Record values and record types can be written inline, with no declaration, and named later without changing call sites.**

1. `let h = (on: add_tax, retries: 3)` builds an anonymous record of type `(on: fn(Int) -> Int, retries: Int)`. Fields are read as always (`h.retries`), and a function-typed field is called as always (`h.on(x)`, spec 3.4).
2. The same syntax works in signatures and returns: `fn minmax(xs: [Int]) -> (lo: Int, hi: Int)` — a tuple whose fields have names, so call sites read `r.lo`, not `r.0`.
3. Anonymous record types match **structurally**: same field names, same types, in any order. Declared `type`s stay **nominal**, as decided in spec 2.8, and keep what anonymity can't give: methods, `invariant`s, `secret` fields, implementing interfaces.
4. `type Handler = (on: fn(Int) -> Int)` declares an alias. An alias is the anonymous type under a name: values flow between the alias and the shape freely. Hardening a prototype means adding the alias, then (when methods or invariants are wanted) turning it into a nominal `type` — the one step where construction sites change.
5. One field, `(on: f)`, is still a record, not a parenthesized expression: the field name makes it one. `()` stays nothing.

## Consequences

- The Python-fast path: write the shape where it's needed, fully checked at compile time; nothing about it is dynamic.
- The TypeScript path into typing, but with contracts, generated tests and capabilities waiting at the named end.
- Two kinds of record types exist: structural (anonymous, no extras) and nominal (declared, full-featured). The split is the same line TypeScript draws between object literals and classes, and spec 2.8 already draws for interfaces.
- The compiler gains interning of structural record types; JSON, printing and generated tests treat anonymous records like records with the same fields.
- Implementation is phased: parser and checker first (chapter 2 spec section), the interpreter, then native code (a shape per distinct anonymous type, as records have today).

## Alternatives considered

- **Structural matching for all records:** would gut nominal identity — invariants and capabilities hang off it. Rejected.
- **Inferred named types (`type` without fields, filled by use):** magic at a distance; the shape wouldn't be readable at the declaration.
- **Plain tuples with positional access only:** already exist; positions don't read well past two fields, and renaming a field is the common edit.
