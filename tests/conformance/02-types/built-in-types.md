# 2. Types: built-in types

Conformance tests for spec section [2.1 Built-in types](../../../docs/spec/02-types/01-built-in-types.md): numbers, their widths and ranges, `Char`, `String` and conversions. Part of the [02-types](README.md) list; format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 2.1 | [explicit-number-conversion.kav](explicit-number-conversion.kav) | Number types convert explicitly with `T(value)`; reading text can fail. | runs |
| 2.1 | [text-literal-conversions-are-checked.kav](text-literal-conversions-are-checked.kav) | Literal text that isn't a valid value of the type is a compile error. | compile error |
| 2.1 | [conversions-of-text-and-characters.kav](conversions-of-text-and-characters.kav) | Text converts to numbers, `Bool` and `Char`; characters by code point. | runs |
| 2.1 | [int-parse-is-gone.kav](int-parse-is-gone.kav) | `Int.parse(text)` is gone: every conversion is `T(value)`. | compile error |
| 2.1 | [no-implicit-number-conversion.kav](no-implicit-number-conversion.kav) | There are no implicit conversions between number types. | compile error |
| 2.1 | [big-constants-are-bigint.kav](big-constants-are-bigint.kav) | A constant that doesn't fit `Int`, with no type written, is an exact `BigInt`. | runs |
| 2.1 | [bigint-arithmetic.kav](bigint-arithmetic.kav) | `BigInt` takes arithmetic, comparisons and `==`, is a map key, and converts. | runs |
| 2.1 | [constants-must-fit-their-type.kav](constants-must-fit-their-type.kav) | A constant that doesn't fit its written type is a compile error. | compile error |
| 2.1, 5.6 | [bigint-division-by-zero-panics.kav](bigint-division-by-zero-panics.kav) | Dividing a `BigInt` by zero panics. | panic |
| 2.1 | [decimal-has-no-size-limit.kav](decimal-has-no-size-limit.kav) | A `Decimal` keeps every digit, however many. | runs |
| 2.1 | [constants-are-exact.kav](constants-are-exact.kav) | A constant of number literals is worked out exactly, then rounded once. | runs |
| 2.1, 5.6 | [int-of-nan-panics.kav](int-of-nan-panics.kav) | A float that isn't a number doesn't fit any whole-number type. | panic |
| 2.1 | [float32-is-single-precision.kav](float32-is-single-precision.kav) | `Float32` rounds to single precision at every step and prints its shortest form. | runs |
| 2.1 | [numbers-read-from-text-at-their-limits.kav](numbers-read-from-text-at-their-limits.kav) | Reading a number from text takes every value its type holds, trims spaces, and gives an error for anything else. | runs |
| 2.1 | [floats-give-the-same-bits-everywhere.kav](floats-give-the-same-bits-everywhere.kav) | A multiply and an add are never fused, and `std.math`'s functions are Kavach's own code, so floats give the same bits on every machine |
| 2.1 | [a-surrogate-is-not-a-char.kav](a-surrogate-is-not-a-char.kav) | a whole number that isn't a character's code point (here a surrogate) panics when converted to `Char`. | panic |
| 2.1 | [a-negative-number-is-not-a-char.kav](a-negative-number-is-not-a-char.kav) | A negative number isn't a character, so converting it to `Char` panics. | panic |
| 2.1 | [a-code-point-past-10ffff-is-not-a-char.kav](a-code-point-past-10ffff-is-not-a-char.kav) | A whole number above `0x10FFFF` isn't a character, so converting it to `Char` panics. | panic |
| 2.1, 1.8, 5.6 | [nan-comparisons-are-false.kav](nan-comparisons-are-false.kav) | an order comparison with `NaN` is false, not a panic: `<`, `<=`, `>` and `>=` on floats never stop the program. | runs |
| 2.1 | [a-float32-literal-must-fit.kav](a-float32-literal-must-fit.kav) | A number literal must fit its type: `1e300` doesn't fit `Float32`. | compile error |
| 2.1, 1.8 | [bit-operators-are-for-integers-only.kav](bit-operators-are-for-integers-only.kav) | The bit operators work on integers only, so `&` on a `Float` is a compile error. | compile error |
| 2.1, 1.8 | [chars-have-no-arithmetic.kav](chars-have-no-arithmetic.kav) | Arithmetic is for numbers: a `Char` has no `+`; its code point is `Int(c)`. | compile error |
| 2.1, 1.8 | [nan-is-not-equal-to-itself.kav](nan-is-not-equal-to-itself.kav) | `NaN` is equal to nothing, itself included, and `-0.0` equals `0.0`; `==` on floats worked out while running warns. | warning |
| 2.1, 1.8 | [plus-joins-text-only-with-text.kav](plus-joins-text-only-with-text.kav) | `+` joins two `String`s; a number is not turned into text by itself. | compile error |
| 2.1, 1.8 | [signed-sized-integers-do-arithmetic.kav](signed-sized-integers-do-arithmetic.kav) | `Int8`, `Int16`, `Int32` and `Int` take `+ - * / % **`; `/` truncates toward zero, `%` keeps the sign of the left side, and each holds its own range. | runs |
| 2.1, 1.8, 5.6 | [uint16-power-overflow-panics.kav](uint16-power-overflow-panics.kav) | A power past `UInt16`'s largest value, 65535, panics. | panic |
| 2.1, 1.8 | [unsigned-integers-do-arithmetic.kav](unsigned-integers-do-arithmetic.kav) | `UInt8`, `UInt16`, `UInt32` and `UInt64` take `+ - * / % **` up to their largest value, and never go below 0. | runs |
| 2.1, 5.6 | [a-bigint-too-big-for-int-panics.kav](a-bigint-too-big-for-int-panics.kav) | Converting a `BigInt` that doesn't fit `Int` panics. | panic |
| 2.1, 1.8 | [a-char-is-not-a-string.kav](a-char-is-not-a-string.kav) | A `Char` is one character, not text: it can't be compared with a `String` or joined to one with `+`. | compile error |
| 2.1, 1.7 | [a-char-literal-holds-one-character.kav](a-char-literal-holds-one-character.kav) | A `Char` literal holds exactly one character; two are text and need double quotes. | compile error |
| 2.1, 5.6 | [a-float-too-big-for-int-panics.kav](a-float-too-big-for-int-panics.kav) | a float past `Int`'s range doesn't fit it, so converting it panics instead of saturating. | panic |
| 2.1, 1.7 | [a-literal-takes-the-type-of-the-other-operand.kav](a-literal-takes-the-type-of-the-other-operand.kav) | A literal next to a sized number takes that number's type, so it must fit it. | compile error |
| 2.1 | [a-negative-literal-does-not-fit-an-unsigned-type.kav](a-negative-literal-does-not-fit-an-unsigned-type.kav) | A negative constant doesn't fit an unsigned type, so writing one there is a compile error. | compile error |
| 2.1, 5.6 | [a-negative-number-to-unsigned-panics.kav](a-negative-number-to-unsigned-panics.kav) | a negative number doesn't fit an unsigned type, so converting it panics. | panic |
| 2.1, 5.6 | [a-number-that-does-not-fit-its-new-type-panics.kav](a-number-that-does-not-fit-its-new-type-panics.kav) | converting a number to a type it doesn't fit panics. | panic |
| 2.1, 2.18 | [a-string-cannot-change-in-place.kav](a-string-cannot-change-in-place.kav) | A `String` never changes: it has no methods that change it in place; a `TextBuilder` is for that. | compile error |
| 2.1 | [a-typed-constant-past-int-is-an-error.kav](a-typed-constant-past-int-is-an-error.kav) | A constant worked out from literals that doesn't fit the type written for it is a compile error, not a `BigInt`. | compile error |
| 2.1, 1.11 | [any-value-converts-to-its-printed-text.kav](any-value-converts-to-its-printed-text.kav) | `String(value)` gives the value's text, as `print` shows it, for every kind of value. | runs |
| 2.1 | [bigint-and-int-do-not-mix.kav](bigint-and-int-do-not-mix.kav) | A `BigInt` and an `Int` can't be added: the `Int` must be converted with `BigInt(n)`. | compile error |
| 2.1 | [bigint-holds-huge-values.kav](bigint-holds-huge-values.kav) | A `BigInt` holds a whole number of any size, negative too, and keeps every digit through arithmetic and comparison. | runs |
| 2.1 | [bigint-holds-very-large-values.kav](bigint-holds-very-large-values.kav) | `BigInt` never overflows: a product of the largest `Int`s, or `2 ** 200`, keeps every digit; `/` truncates and `%` keeps the left side's sign, as for `Int`. | runs |
| 2.1, 2.13 | [bytes-cannot-change.kav](bytes-cannot-change.kav) | `Bytes` are immutable: a byte can't be set, even in a `let`. | compile error |
| 2.1, 1.7, 1.8 | [chars-hold-any-unicode-character.kav](chars-hold-any-unicode-character.kav) | A `Char` holds any one Unicode character, written plainly or with `\u{…}`; it converts to and from its code point and orders by it. | runs |
| 2.1 | [decimal-and-float-do-not-mix.kav](decimal-and-float-do-not-mix.kav) | A `Decimal` and a `Float` can't be added: there are no implicit number conversions. | compile error |
| 2.1 | [decimal-arithmetic-is-exact-while-running.kav](decimal-arithmetic-is-exact-while-running.kav) | `Decimal` arithmetic is exact base 10 while the program runs too: 0.1 + 0.2 is 0.3, unlike `Float`. | runs |
| 2.1 | [float-and-float32-do-not-mix.kav](float-and-float32-do-not-mix.kav) | Arithmetic between `Float` and `Float32` is a compile error: no implicit conversions. | compile error |
| 2.1 | [float-to-whole-number-cuts-the-fraction.kav](float-to-whole-number-cuts-the-fraction.kav) | a float or `Decimal` becomes a whole number by cutting off the fraction, toward zero. | runs |
| 2.1, 1.8 | [int-and-float-do-not-compare.kav](int-and-float-do-not-compare.kav) | An `Int` and a `Float` can't be compared with `==` or `<` without a conversion. | compile error |
| 2.1 | [int-and-float-do-not-mix.kav](int-and-float-do-not-mix.kav) | An `Int` and a `Float` can't be added: there are no implicit number conversions. | compile error |
| 2.1, 5.6 | [int-overflow-at-its-minimum-panics.kav](int-overflow-at-its-minimum-panics.kav) | Going below `Int`'s smallest value panics. | panic |
| 2.1, 5.6 | [int16-overflow-at-its-maximum-panics.kav](int16-overflow-at-its-maximum-panics.kav) | Going above `Int16`'s largest value, 32767, panics. | panic |
| 2.1, 5.6 | [int32-multiply-overflow-panics.kav](int32-multiply-overflow-panics.kav) | A product past `Int32`'s range panics. | panic |
| 2.1, 5.6 | [int8-overflow-at-its-minimum-panics.kav](int8-overflow-at-its-minimum-panics.kav) | Going below `Int8`'s smallest value, -128, panics. | panic |
| 2.1, 1.11 | [negative-zero-and-infinities.kav](negative-zero-and-infinities.kav) | floats follow IEEE 754: zero has a sign that prints, dividing by a signed zero gives an infinity of that sign, and `Float32` overflows to infinity without a panic. | runs |
| 2.1 | [numbers-convert-between-floats-and-decimal.kav](numbers-convert-between-floats-and-decimal.kav) | Any number converts to any other with `T(value)`: whole numbers to floats and `Decimal`, a `Decimal` to a float, a float to a `Decimal`. | runs |
| 2.1 | [reading-text-as-a-number-needs-try-or-catch.kav](reading-text-as-a-number-needs-try-or-catch.kav) | Converting text that isn't a literal to a number can fail, so it must be handled with `try`, `catch` or `match`. | compile error |
| 2.1 | [sized-integers-do-not-mix.kav](sized-integers-do-not-mix.kav) | Two different sized integers can't be added: there are no implicit number conversions. | compile error |
| 2.1, 1.7 | [sized-integers-hold-their-full-range.kav](sized-integers-hold-their-full-range.kav) | Each sized integer type takes every literal from its smallest to its largest value. | runs |
| 2.1, 1.11 | [text-converts-to-decimal-keeping-its-digits.kav](text-converts-to-decimal-keeping-its-digits.kav) | `Decimal(text)` reads the text exactly, keeping its scale; `BigInt(text)` reads a whole number of any size; other text fails. | runs |
| 2.1 | [text-literal-that-does-not-fit-is-a-compile-error.kav](text-literal-that-does-not-fit-is-a-compile-error.kav) | A literal text read into a sized number is checked while compiling: `Int8("300")` doesn't fit, so it is a compile error. | compile error |
| 2.1 | [text-to-bool-reads-only-true-or-false.kav](text-to-bool-reads-only-true-or-false.kav) | `Bool(text)` reads `true` or `false`, with spaces around allowed; any other text fails. | runs |
| 2.1, 5.6 | [uint32-overflow-at-its-maximum-panics.kav](uint32-overflow-at-its-maximum-panics.kav) | Going above `UInt32`'s largest value, 4294967295, panics. | panic |
| 2.1, 5.6 | [uint64-overflow-at-its-maximum-panics.kav](uint64-overflow-at-its-maximum-panics.kav) | Going above `UInt64`'s largest value panics. | panic |
| 2.1, 5.6 | [uint8-below-zero-panics.kav](uint8-below-zero-panics.kav) | An unsigned number can't go below 0: `UInt8` 0 - 1 panics. | panic |
| 2.1 | [unsigned-and-signed-do-not-compare.kav](unsigned-and-signed-do-not-compare.kav) | Comparing two different number types is a compile error: no implicit conversions. | compile error |
| 2.1 | [whole-numbers-convert-exactly-between-types.kav](whole-numbers-convert-exactly-between-types.kav) | A whole number converts exactly to any whole-number type it fits, `BigInt` included, and back. | runs |
| 2.1, 5.6 | [a-sized-remainder-by-zero-panics.kav](a-sized-remainder-by-zero-panics.kav) | Whole-number remainder by zero panics for a sized type too, here `UInt8`. | panic |
| 2.1, 5.6 | [float-division-by-zero-does-not-panic.kav](float-division-by-zero-does-not-panic.kav) | only whole-number division by zero panics; a float divided by zero is an IEEE infinity or `NaN`. | runs |
| 2.1, 5.6 | [a-uint-of-12-bits-panics-past-4095.kav](a-uint-of-12-bits-panics-past-4095.kav) | A sum that doesn't fit an `UInt<bits: 12>` panics, as overflow does. | panic |
| 2.1, 5.6 | [a-uint-of-12-bits-wraps-when-asked.kav](a-uint-of-12-bits-wraps-when-asked.kav) | `wrapping_add` and the other wrapping methods wrap at the type's width; `checked_add` gives `none` past it. | runs |
| 2.1 | [an-int-of-24-bits-holds-its-sign.kav](an-int-of-24-bits-holds-its-sign.kav) | An `Int<bits: 24>` holds -8,388,608 to 8,388,607, and keeps its sign through arithmetic. | runs |
| 2.1, 5.6 | [an-int-of-24-bits-panics-below-its-smallest.kav](an-int-of-24-bits-panics-below-its-smallest.kav) | A result below an `Int<bits: 24>`'s smallest value panics. | panic |
| 2.1, 5.6 | [a-negated-smallest-int-of-bits-panics.kav](a-negated-smallest-int-of-bits-panics.kav) | The smallest `Int<bits: n>` has no positive twin: negating it panics. | panic |
| 2.1, 1.8 | [bit-operators-work-at-the-width.kav](bit-operators-work-at-the-width.kav) | The bit operators work on a `bits` number; `~` flips its `n` bits. | runs |
| 2.1, 1.8, 5.6 | [a-shift-past-the-width-panics.kav](a-shift-past-the-width-panics.kav) | A shift of a `bits` number takes 0 to `n - 1`. | panic |
| 2.1, 2.9 | [a-literal-too-big-for-its-bits-is-an-error.kav](a-literal-too-big-for-its-bits-is-an-error.kav) | A literal that doesn't fit a `bits` type is a compile error. | compile error |
| 2.1 | [int8-is-int-of-8-bits.kav](int8-is-int-of-8-bits.kav) | `Int8` … `UInt64` are short spellings of `bits` types, and `Int` is `Int<bits: 64>`: each pair is one type. | runs |
| 2.1, 5.6 | [a-conversion-into-bits-that-does-not-fit-panics.kav](a-conversion-into-bits-that-does-not-fit-panics.kav) | `UInt<bits: 12>(x)` converts any number, cutting a float to its whole part, and panics when it doesn't fit. | panic |
| 2.1, 2.9 | [try-into-bits-fails-with-a-size-error.kav](try-into-bits-fails-with-a-size-error.kav) | Under `try` or `catch`, a number that doesn't fit its whole-number type fails with a `SizeError`. | runs |
| 2.1 | [try-int8-of-a-number-fails-with-a-size-error.kav](try-int8-of-a-number-fails-with-a-size-error.kav) | `Int8` is a `bits` type, so a number converted to it under `catch` fails with a `SizeError` too. | runs |
| 2.1, 5.6 | [a-sum-of-bits-numbers-is-checked-at-the-width.kav](a-sum-of-bits-numbers-is-checked-at-the-width.kav) | A sum of `bits` numbers panics when a partial sum doesn't fit the width. | panic |
| 2.1, 2.12 | [bits-numbers-go-in-records-lists-and-json.kav](bits-numbers-go-in-records-lists-and-json.kav) | `bits` and range numbers print, sort, go in lists, dicts and records, and are written to JSON as numbers. | runs |
| 2.1 | [a-width-is-from-1-to-65535-bits.kav](a-width-is-from-1-to-65535-bits.kav) | A width is from 1 to 65,535 bits. | compile error |
| 2.1 | [uint-is-written-with-its-bits.kav](uint-is-written-with-its-bits.kav) | `UInt` is always written with its bits. | compile error |
| 2.1, 5.6 | [a-range-keeps-its-range-through-arithmetic.kav](a-range-keeps-its-range-through-arithmetic.kav) | Arithmetic on a range gives the range; a result out of it panics. | panic |
| 2.1 | [a-literal-outside-a-range-is-an-error.kav](a-literal-outside-a-range-is-an-error.kav) | A literal given as a range value must be in the range. | compile error |
| 2.1 | [a-literal-beside-a-range-need-only-be-an-int.kav](a-literal-beside-a-range-need-only-be-an-int.kav) | A literal on the right of an operator or a comparison on a range need only be an `Int`: the result is checked. | runs |
| 2.1, 5.6 | [checked-add-on-a-range-gives-none.kav](checked-add-on-a-range-gives-none.kav) | `checked_add` on a range gives `none` when the sum is out of the range. | runs |
| 2.1 | [wrapping-on-a-range-is-an-error.kav](wrapping-on-a-range-is-an-error.kav) | Wrapping has no meaning for a range: `wrapping_add` on one is a compile error. | compile error |
| 2.1 | [bit-operators-on-a-range-are-errors.kav](bit-operators-on-a-range-are-errors.kav) | The bit operators work on bits, not on a range: they are compile errors on one. | compile error |
| 2.1, 5.6 | [a-conversion-into-a-range-that-does-not-fit-panics.kav](a-conversion-into-a-range-that-does-not-fit-panics.kav) | `Int<range: a..=b>(n)` panics when `n` is out of the range; `Int(p)` converts back. | panic |
| 2.1, 2.9 | [try-into-a-range-fails-with-a-size-error.kav](try-into-a-range-fails-with-a-size-error.kav) | `try Int<range: a..=b>(n)` fails with a `SizeError` when `n` is out of the range. | runs |
| 2.1 | [a-range-up-to-its-end-leaves-the-end-out.kav](a-range-up-to-its-end-leaves-the-end-out.kav) | `Int<range: a..b>` leaves `b` out: it is the type `Int<range: a..=b-1>`. | panic |
| 2.1 | [an-empty-range-type-is-an-error.kav](an-empty-range-type-is-an-error.kav) | A range type with no values is a compile error. | compile error |
