# The Classic Mistakes of Other Languages

Research done 2026-11-03, at Mithlesh's request: "`0.1 + 0.2 = 0.30000000000000004` etc. — fix those in Kavach, one more reason industry chooses it." Where [bug-classes.md](bug-classes.md) covers the *security and reliability* bug classes (memory, injection, nulls), this covers the *everyday correctness traps*: numbers, text, collections and time, where a program runs, gives an answer, and the answer is wrong.

Each trap was checked by running Kavach, not by reading the spec. Status:

- **Design**: Kavach rules it out by how the language is built.
- **Fixed**: fixed in Kavach on 2026-11-03, with a spec rule and a conformance test.
- **Proposal**: a fix that changes a default or costs speed; it waits for a maintainer's review.

## Numbers

| Trap | Elsewhere | Kavach |
|---|---|---|
| `0.1 + 0.2` is `0.30000000000000004` | Every language with IEEE 754 floats: C, Java, JavaScript, Python, Rust ([Goldberg 1991](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html)). Go computes *constants* exactly and rounds once ([Go blog](https://go.dev/blog/constants)) | **Fixed** for constants ([decision 0043](../decisions/0043-constants-are-exact.md)): `0.1 + 0.2` prints `0.3`, `1.0 - 0.9` prints `0.1`, `100 * 1.1` prints `110.0`. Values worked out while running still round at each step, as on every machine |
| `a == b` on computed floats | Silently false; clippy's `float_cmp` lint warns in Rust | **Fixed**: a warning (spec 1.8) and `a.near(b)` (14.2). No warning for whole numbers, constants or `example` lines |
| Adding `0.1` ten times isn't `1.0` | Everywhere. The Patriot missile failure at Dhahran (1991, 28 deaths) came from 0.1 s ticks summed in limited precision for 100 hours ([GAO IMTEC-92-26](https://www.gao.gov/products/imtec-92-26)) | **Design** for money: `Decimal` is exact, and now has no size limit ([0042](../decisions/0042-decimal-has-no-size-limit.md)). **Proposal 2** for sums of floats |
| `NaN` in a list breaks sorting | C++ `std::sort` has undefined behaviour; Java and JavaScript leave the list half sorted; Python's `sorted([3, nan, 1])` gives `[3, nan, 1]` | **Fixed**: `NaN` sorts after every number, in `sort`, `sort_by`, `min` and `max` (14.2). Before, Kavach left `[3.0, NaN, 1.0]` unsorted too |
| `NaN` converted to a whole number | C: undefined behaviour; Java gives `0`; Rust saturates to `0` | **Fixed**: `Int(NaN)` panics, as `Int(1e300)` does. The interpreter gave `0` before; native code already panicked |
| Integers wrap around silently | C, C++, Java, Go, and Rust in release builds. The binary search in the JDK broke for arrays over 2³⁰ items because `(low + high) / 2` overflowed ([Google Research](https://research.google/blog/extra-extra-read-all-about-it-nearly-all-binary-searches-and-mergesorts-are-broken/)) | **Design**: overflow is checked always (5.6) |
| A constant too big for its type | C truncates `int8_t x = 300` to 44 with a warning at best | **Fixed**: `const x: Int8 = 300` is a compile error that names the range and a type that fits; a constant too big for `Int` with no type written is a `BigInt` ([0041](../decisions/0041-bigint-is-built-in.md)) |
| A float squeezed into a small integer | Ariane 5 flight 501 (1996): a 64-bit float converted to a 16-bit integer overflowed, and the unhandled error destroyed the rocket ([inquiry report](https://esamultimedia.esa.int/docs/esa-x-1819eng.pdf)) | **Design**: no implicit conversions; `Int16(x)` is written out, and panics with the value and the type instead of giving a wrong number |
| `x % 2 == 1` to test for odd | False for negative odd `x` in C, Java, JavaScript, Go and Rust, where `-3 % 2` is `-1` (Python floors, giving `1`) | **Fixed**: a warning that says to write `x % 2 != 0` (spec 1.8). **Proposal 4** for floor division |
| `round(2.5)` | Python rounds half to even (`2`); JavaScript and C round half up or away from zero (`3`) | `math.round` rounds half away from zero (`3`), as written in 14.5. **Proposal 5** for money |
| `"1" + 1`, `"10" * "2"` | JavaScript coerces: `"11"`, `20` | **Design**: static types, no coercion |
| `parseInt("23sd")` is 23; `parseInt("08")` was 0 in old JavaScript | JavaScript | **Fixed**: `Int(text)` reads the whole text strictly and fails on `"23sd"`; a literal is checked while compiling. Finding a number *inside* text is `text.find_int`, with fixed rules (14.9) |

## Text

| Trap | Elsewhere | Kavach |
|---|---|---|
| `"😀".length` is 2 | JavaScript, Java and C# count UTF-16 units ([MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/length)) | **Design**: `len()` counts characters: `"😀".len()` is 1 |
| `"é" == "é"` is false when one is written `e` + accent | Every language that compares code points ([UAX #15](https://unicode.org/reports/tr15/)) | Same in Kavach today. **Proposal 3** |
| `"ß".upper()` | Some libraries give `"ß"` | `"SS"`, as Unicode says |
| `"10" < "9"` | True everywhere: text compares character by character | Same, as it should be for text; numbers compare as numbers because types are static |
| Sorting numbers as text: `[10, 9, 1].sort()` gives `[1, 10, 9]` | JavaScript's default `sort` ([MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/sort)) | **Design**: `[10, 9, 1, 2].sort()` is `[1, 2, 9, 10]` |

## Values and collections

| Trap | Elsewhere | Kavach |
|---|---|---|
| Two names for one list: changing one changes the other | Python, JavaScript, Java ([Python gotchas](https://docs.python-guide.org/writing/gotchas/)) | **Design**: values, copied on write (spec 6.1) |
| A default argument shared between calls: `def f(x, items=[])` | Python | **Design**: defaults are worked out at each call. `add(1)` then `add(2)` give `[1]` and `[2]` |
| A closure seeing the loop variable's last value | JavaScript with `var`, Go before 1.22 | **Design**: closures capture copies |
| `null` where a value was expected | Java, JavaScript, C# | **Design**: optionals (2.7) |
| Two maps with the same entries compare unequal because of order | Some libraries | **Fixed** ([0039](../decisions/0039-equality-is-deep-and-fast.md)): order doesn't count in `==` |
| `a < b < c` compares a boolean with `c` | C, Java, JavaScript | **Fixed** ([0040](../decisions/0040-chained-comparisons.md)): it means `a < b and b < c` |
| `try`/`catch` around code that can't fail | Silently does nothing | **Fixed**: a warning (spec 5.2) |

## Time

`std.time` has only clocks and durations today (14.4), so the date traps can be avoided before dates arrive:

- **Months from 0** (JavaScript's `new Date(2024, 1, 1)` is February): months will be 1 to 12.
- **The 1900 leap year** (Excel treats 1900 as one, for Lotus 1-2-3 compatibility): dates will follow the proleptic Gregorian calendar, with no exceptions.
- **"Local time" by default**: a date and time without a zone will be a different type from one with a zone, so they can't be mixed by accident. Daylight saving gaps and overlaps (02:30 that happens twice, or never) will be values the program must handle, not silent guesses.
- **Leap seconds and clocks going back**: `time.monotonic_ms()` already exists for measuring; durations should never come from wall-clock differences.

These go into the spec when dates are designed.

## Proposals for the maintainers

Decided (2026-11-03, [decision 0051](../decisions/0051-number-and-text-defaults-keep-their-speed.md)): 1, 2 and 3 keep today's defaults, for speed and stable meaning; 4 and 5 are added as methods (spec 14.2).

1. **`0.1` is a `Decimal` where no type is written.** Run-time sums of such numbers would then be exact too (`let a = 0.1; a + 0.2` is `0.3`). It costs speed: `Decimal` arithmetic is several times slower than `Float`, and every unannotated number with a point would be one. Scientific code would write `Float` or `0.1f`.
2. **Accurate float sums.** `xs.sum(…)` on floats could add with Neumaier's compensated summation, which makes ten `0.1`s exactly `1.0` and keeps the error of a million additions near one rounding instead of growing. It costs about 2 to 4 times the work per item; pairwise summation (NumPy's way) is nearly free but less exact.
3. **Text equality after Unicode normalization.** `==` on text would compare the NFC forms, so `"é"` written either way is equal. It costs a check per comparison (a fast path when both are already NFC, which nearly all text is), and changes the meaning of `==`.
4. **Division and remainder that round down.** `-7 / 2` is `-3` and `-7 % 3` is `-1` in Kavach, as in C and Java; Python gives `-4` and `2`. Changing `/` and `%` would surprise C and Java readers. Adding `x.div_floor(n)` and `x.mod(n)` (always 0 or more) changes nothing.
5. **Rounding for money.** `Decimal` could get `d.round(places)` with half-to-even ("banker's rounding", the default in accounting and in IEEE 754) and `d.round_half_up(places)`, named so the choice is visible.
