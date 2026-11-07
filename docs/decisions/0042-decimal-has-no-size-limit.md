# 0042. `Decimal` has no size limit

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

`Decimal` held its digits in 128 bits, about 38 of them. A bigger or more precise value panicked with `decimal overflow`, and a longer literal silently became 0. Mithlesh asked for big decimals too (2026-11-03). Of two ways, a separate `BigDecimal` type or `Decimal` itself growing, Mithlesh chose the second.

## Decision

**A `Decimal` keeps every digit, however many (spec 2.1).**

1. A value that fits 128 bits keeps the fast form it always had. One that doesn't is held as `num-bigint` digits with a scale (decision 0041), and arithmetic on it is exact.
2. A result that fits the fast form again goes back to it. So each value has one form, and equal values hash alike.
3. Division that doesn't end keeps 18 places after the point, in both forms, as before.
4. Nothing changes for code: same type, same operators, same printing. `decimal overflow` no longer happens, and a long literal keeps its value.

## Consequences

- Money code stays as fast as before: the big form appears only for values no money amount reaches.
- The big form's code is linked only into programs that make one (decision 0024).

## Alternatives considered

- **A separate `BigDecimal` type:** `Decimal` would never change, but there'd be one more number type to learn, and code would need converting between the two.
- **Always the big form:** simpler, but every money sum would allocate.
