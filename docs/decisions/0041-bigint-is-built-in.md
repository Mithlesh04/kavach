# 0041. `BigInt` is a built-in number type, and big constants infer it

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

`let d = 234234 * 23424293429030023` panicked with an integer overflow. Mithlesh asked Kavach to handle it when no type is written (2026-11-03). Spec 2.1 already promised a `BigInt` for whole numbers of any size, but it didn't exist.

Three ways were weighed, and Mithlesh chose the first:

1. **Infer `BigInt`** for a constant that doesn't fit `Int`. Run-time `Int` stays 64 bits.
2. Make it a compile-time error with a hint to write `BigInt`.
3. Let every `Int` grow when it overflows, as Python does. Every `Int` would need a tag, so lists of numbers would lose their packed form, and every sum would get slower.

For `a * b` and `a < b` to work, the compiler must know the type. A type written in Kavach in `std.math` couldn't have operators, because Kavach has no operator overloading.

## Decision

**`BigInt` is built in, like `Decimal` (spec 2.1). Its arithmetic is the `num-bigint` crate.**

1. `BigInt` takes `+ - * / % **`, comparisons, equality and hashing, and prints as its digits.
2. As with every number type, there are no implicit conversions: `BigInt(n)` makes one from any whole number, and a literal takes the type its place expects.
3. **A constant that doesn't fit `Int` is a `BigInt` when no type is written.** This covers a literal, or `+ - * / % **` and a minus on literals. The compiler works out the exact value. Where a type is written (`const x: Int = 2 ** 70`), it is a compile error.
4. Values worked out at run time stay as typed: `Int` arithmetic still panics on overflow (5.6).
5. `num-bigint` (MIT or Apache-2.0) is an outside dependency, approved by Mithlesh on 2026-11-03. It is used by `runtime/core`, so the interpreter and native code compute the same results.

## Consequences

- `let d = 234234 * 23424293429030023` prints `5486765947055418407382`.
- `Int` keeps its speed: nothing changes for code that never writes a huge constant or `BigInt`.
- This is an exception to the rule that standard library modules need no compiler change. It's made because the compiler itself infers this type.

## Alternatives considered

- **`BigInt` in `std.math`, methods only** (`a.mul(b)`): no compiler change, but no operators.
- **Operator interfaces** (`Add`, `Compare`) for any type: general, but a much bigger language change. It can come later; `BigInt` would then keep working the same.
- **Our own big-number code:** no dependency, but slower on numbers with thousands of digits, and more to test.
