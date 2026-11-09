# 0043. Constants are worked out exactly, then rounded once

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

`print(0.1 + 0.2)` printed `0.30000000000000004`, in Kavach as in C, Java, JavaScript and Python. Mithlesh asked for the classic mistakes of other languages to be fixed in Kavach (2026-11-03).

The cause is that each literal is rounded to a binary `Float` first (0.1 becomes 0.1000000000000000055…), and then the sum is rounded again. Go avoids this for constants: an untyped constant expression is computed exactly, and rounded to `float64` once, at the end. In Go, `fmt.Println(0.1 + 0.2)` prints `0.30000000000000004` only when the values are variables.

Decision 0041 already works out whole-number constants exactly, to find those that need a `BigInt`.

## Decision

**A constant of number literals (`+ - * /`, a minus, and `**` with a whole exponent) is worked out exactly, then rounded once to its type (spec 2.1).**

1. `0.1 + 0.2` is the `Float` nearest 0.3, so it prints `0.3`, and `0.1 + 0.2 == 0.3` is `true`.
2. Only constants change. Values worked out while the program runs are rounded at each step, as IEEE 754 requires and as every machine does: `let a = 0.1` then `a + 0.2` is still `0.30000000000000004`. Spec 1.8 warns where that bites.
3. A constant whose type is `Decimal` was exact already. Whole-number constants follow decision 0041.

## Consequences

- The best-known example of floating-point surprise gives the answer people expect.
- A constant gives the same value however it is written: `0.3`, `0.1 + 0.2` and `3 / 10.0` are one `Float`.

## Alternatives considered

- **`0.1` is a `Decimal` when no type is written:** fixes run-time sums too, but every unannotated decimal number would be slower. That's a change to a default, so it waits for Mithlesh's review (`docs/vision/classic-mistakes.md`).
- **Printing floats rounded to fewer digits:** hides the error without removing it; two values that print alike would compare unequal.
