# 0051. Number and text defaults keep their speed

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

The research into the classic mistakes of other languages ([classic-mistakes.md](../vision/classic-mistakes.md)) ended with five proposals that change a default or add a method. Mithlesh asked for a decision on each, weighed against Kavach's aims: as fast as C++, simple to write, and clear to read.

## Decision

1. **`0.1` stays a `Float` where no type is written.** A `Decimal` default would make every number with a point several times slower. Constants are already exact ([decision 0043](0043-constants-are-exact.md)), so `0.1 + 0.2 == 0.3`; money uses `Decimal`, which is exact.
2. **A float sum adds in its order, as plain additions.** Compensated summation costs two to four times the work per item, and the same order everywhere keeps results the same on every core and on the GPU, as Mithlesh decided for GPU sums.
3. **`==` on text compares code points.** Normalizing first adds a check to every comparison and changes what `==` means. Text from outside can be normalized on purpose, when the Unicode functions of `std.text` are designed.
4. **`x.div_floor(n)` and `x.mod(n)` are added** for whole numbers: division that rounds down, and a remainder that is never negative when `n` is positive (`(-7).div_floor(2)` is `-4`, `(-7).mod(3)` is `2`). `/` and `%` stay as in C and Java.
5. **`d.round(places)` and `d.round_even(places)` are added** for `Decimal`. `round` rounds half away from zero, like `math.round` (14.5), so one name means one rule everywhere; `round_even` rounds half to even, the accountants' rule, named so the choice shows.

## Consequences

- No program gets slower, and no existing program changes meaning.
- Code that needs floor division, a non-negative remainder or banker's rounding says so in one call.
- Unicode normalization waits for the `std.text` Unicode work.

## Alternatives considered

- Each of the five changes to defaults as proposed: see the costs above. Speed and stable meaning come first; exactness is one method or one type away.
