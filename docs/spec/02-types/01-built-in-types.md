# 2.1 Built-in types

| Type | Meaning |
|---|---|
| `Int` | 64-bit signed integer |
| `Int8`, `Int16`, `Int32`, `UInt8`, `UInt16`, `UInt32`, `UInt64` | Sized integers; any other width is `Int<bits: n>` or `UInt<bits: n>`, and a range is `Int<range: a..=b>` (below) |
| `Float` | 64-bit floating point |
| `Float32` | 32-bit floating point: rounded to single precision at every step, and printed as the shortest text that reads back as the same `Float32` (`0.1`). What GPUs compute fastest (chapter 15) |
| `Decimal` | Exact base-10 number, with as many digits as it needs. Use it for money; `Decimal<digits: 10, places: 2>` fixes the places (below; *not built yet*) |
| `BigInt` | A whole number of any size. Slower than `Int`: use it when values may not fit in 64 bits |
| `Bool` | `true` or `false` |
| `Char` | One Unicode character |
| `String` | Immutable UTF-8 text |
| `Bytes` | Immutable raw bytes |
| `Json` | A JSON value whose shape is known only when the program runs (2.12) |

There are **no implicit conversions** between number types. Every conversion has one form, the type's name called with the value: `T(value)`.

```kavach
const n: Int = 7
const f = Float(n)                   // always succeeds
const parsed = try Int(text)         // text may not be a number, so it can fail (chapter 5)
const two = Int("2")                 // a literal is checked while compiling: just 2
```

| From → to | Result |
|---|---|
| Any number → any number | Whole numbers convert exactly. A float or `Decimal` becomes a whole number by cutting off the fraction (`Int(2.7)` is `2`). A value that doesn't fit the target panics (`Int8(300)`, 5.6) |
| Any value → `String` | Its text, as `print` shows it (`String(2.5)` is `"2.5"`) |
| `String` → a number, `Bool` or `Char` | The text read whole, with spaces around it allowed. It **can fail** with a `ParseError`, so it takes `try` or `catch`. `Int("23sd")` fails, as does `Int("300")` into `Int8`. `Bool` reads `true` or `false`; `Char` reads text of exactly one character, trimming nothing: `Char(" ")` is a space, and `Char(" k ")` fails |
| `Char` → a whole number | The character's Unicode code point: `Int('a')` is `97` |
| A whole number → `Char` | The character with that code point. One that isn't a character (a negative number, a surrogate, above `0x10FFFF`) panics |

A number literal must fit its type: `const x: Float32 = 1e300` is a compile error, as `Float32` reaches only about `3.4e38`. A value worked out while the program runs that passes a float's largest is `inf`, as IEEE 754 says: `Float32(1e300)` of a `Float` variable is `inf`.

When the text is a literal, the compiler reads it itself: `Int("2")` is the number `2` and can't fail, and `Int("abc")` is a compile error.

To find a number inside other text (`"order 23 shipped"`), use `std.text` (14.9).

**`Decimal` has no size limit** ([decision 0042](../../decisions/0042-decimal-has-no-size-limit.md)): a value of up to about 38 digits is held in a fast 128-bit form, and a bigger or more precise one keeps all its digits in a bigger form, with no change to the code. A division that doesn't end keeps 18 digits after the point.

```kavach
const rate: Decimal = 0.000000000000000000000000000001
const huge: Decimal = 99999999999999999999999999999999999999.99
print(huge + rate)        // 99999999999999999999999999999999999999.990000000000000000000000000001
```

**Constants are exact** ([decision 0043](../../decisions/0043-constants-are-exact.md)): a constant of number literals (`+ - * /`, a minus, and `**` with a whole exponent) is worked out exactly while compiling, then rounded once to its type. A constant that comes to zero keeps the sign floats give it, so `-0.0` is negative zero (`1.0 / -0.0` is `-inf`). Values worked out while the program runs are rounded at each step, as on every machine.

```kavach
print(0.1 + 0.2)             // 0.3: one constant, rounded once
print(0.1 + 0.2 == 0.3)      // true
let a = 0.1
print(a + 0.2)               // 0.30000000000000004: worked out at run time
```

**`BigInt`** holds a whole number of any size ([decision 0041](../../decisions/0041-bigint-is-built-in.md)). It takes `+ - * / % **`, comparisons and `==`, works as a dict key, and prints as its digits. `BigInt(n)` makes one from any whole number, and a literal takes the type its place expects.

A constant that doesn't fit `Int` is a `BigInt` when no type is written: a whole-number literal, or `+ - * / % **` and a minus on literals. The compiler works out its exact value. Where a type is written, a constant that doesn't fit it is a compile error. Values worked out while the program runs keep their type, so `Int` arithmetic still panics on overflow (5.6).

```kavach
let d = 234234 * 23424293429030023    // a BigInt: 5486765947055418407382
const big = 2 ** 100                  // a BigInt
const n = BigInt(7) ** 30             // BigInt(…) from an Int
print(d + 1)                          // 1 is a BigInt here
const x: Int = 2 ** 70                // error: doesn't fit in Int
```

**Floats give the same bits everywhere** ([decision 0122](../../decisions/0122-floats-give-the-same-bits-everywhere.md)). A `Float` or `Float32` program gives the same results, to the last bit, on every machine, system and build, interpreted or native. Each `+ - * /` is rounded on its own, as IEEE 754 says: a multiply and an add are never fused into one step, and a sum written with `+` keeps its order. A list's `sum` and `mean` of floats are exact, rounded once (14.2), so they give the same bits in any order. `std.math`'s functions are Kavach's own code, the same on every system (14.5). So a game's simulation or a test's expected number doesn't change from Windows to Linux. Only the GPU is apart: its own hardware works out `gpu fn`s (chapter 15).

```kavach
let a = 1.1
let b = 2.2
print(a * b + 3.3)        // 5.720000000000001 everywhere: a multiply, then an add
print(math.sin(1e22))     // -0.8522008497671888 everywhere
```

**Any width: `Int<bits: n>` and `UInt<bits: n>`** ([decision 0137](../../decisions/0137-numbers-in-types.md), 2.9). A whole number of exactly `n` bits, signed or not, for hardware registers, chip design and file formats. `n` is from 1 to 65,535, as in Zig. The sized names are short spellings of these: `Int8` is `Int<bits: 8>`, `UInt64` is `UInt<bits: 64>`, and `Int` is `Int<bits: 64>`: each pair is one type. `UInt` is always written with its bits. The width is a literal or a constant known while compiling.

```kavach
const addr: UInt<bits: 12> = 4095          // the largest 12-bit number
const temp: Int<bits: 24> = -8_388_608     // the smallest 24-bit number
const over: UInt<bits: 12> = 4096          // compile error: 4096 doesn't fit in 12 bits
let pc: UInt<bits: 12> = 4090
pc = pc.wrapping_add(10)                   // 4: wraps at 12 bits, as a register does
const wide: UInt<bits: 128> = 2 ** 100     // fits: 128 bits are enough
```

- `+ - * / % **`, the comparisons and the bit operators work as on the other whole numbers (1.8). Both sides have the same type, and a result that doesn't fit panics (5.6): `integer overflow: 4095 + 1 doesn't fit in UInt<bits: 12>`. `wrapping_add` and `checked_add` (5.6) work at the type's width; `~` flips its `n` bits; `<<` and `>>` take a shift from 0 to `n - 1`.
- `UInt<bits: 12>(x)` converts any number, and panics when `x` doesn't fit, as `Int8(300)` does: `5000 doesn't fit in UInt<bits: 12>`. A float is cut to its whole part first.
- Under `try` or `catch`, a conversion of a number into any whole-number type fails with a `SizeError` instead of panicking (2.9), with the same text. This holds for `Int8` … `UInt64` too, as they are `bits` types: `try Int8(n)`. Text read as a number still fails with a `ParseError`.
- It prints, compares, goes in lists, dicts and records, and is written to JSON as the number it is. Read from JSON, a number that doesn't fit is an error (2.12), wherever it is in the type: ``a whole number that fits `UInt<bits: 12>` ``.
- A type wider than 64 bits, as `Int<bits: 128>` or `UInt<bits: 256>`, holds its value as a `BigInt` does, so each operation on it is slower than on 64 bits or fewer. Everything above holds for it: a result that doesn't fit panics, and `wrapping_add`, `~`, `<<` and the other bit operators work at its width.
- *Not built yet:* A value of 64 bits or fewer is kept in the smallest of 1, 2, 4 or 8 bytes that holds it. Today every one is kept in 8 bytes, as an `Int`.
- A generic function given one checks its type's limits too: `twice(UInt<bits: 12>(3000))`, with `fn twice<T: Number>(x: T) -> T { x + x }`, panics as `3000 + 3000` does.

**A range: `Int<range: a..=b>`.** A whole number that is always in the range: an `Int` whose smallest and largest are `a` and `b`. Both ends are known while compiling; `a..b` leaves `b` out, as a range does (1.8), so `Int<range: 0..10>` is `Int<range: 0..=9>`, one type, and is shown that way. An empty range is a compile error.

```kavach
const port: Int<range: 1..=65535> = 8080
const none_yet: Int<range: 1..=65535> = 0       // compile error: 0 is not in 1..=65535

fn next(p: Int<range: 1..=65535>) -> Int<range: 1..=65535> {
    p + 1                                        // panics when p is 65535
}

fn port_of(n: Int) -> Int<range: 1..=65535> {
    Int<range: 1..=65535>(n)                     // panics when n is out of the range
}
```

- Arithmetic of two values of one range type gives that type; a result out of the range panics, as overflow does (5.6): `integer overflow: 65535 + 1 doesn't fit in Int<range: 1..=65535>`. `checked_add` gives `none`. Wrapping has no meaning for a range: `wrapping_add` on one is a compile error. `sum` adds with `+`, so a sum of range values is that range type too, and a total past the range panics; to add up counts that are each in a range, widen each one: `sales.sum(s => Int(s.units))`.
- A literal given as a range value must be in the range: `const none_yet` above is an error. A literal on the right of an operator or a comparison need only be an `Int`, as the result is what is checked: `p - 1` and `p > 0` are allowed when `1` and `0` are not in the range.
- The bit operators `& | ^ << >> ~` work on bits, not on values in a range: they are compile errors on a range. `Int(p) & 1` converts first.
- Two ranges are two types, and a range is not a `bits` type: `Int<range: 0..=255>` is not `UInt8`. `Int(port)` always converts back. `Int<range: 1..=65535>(n)` converts, and panics when `n` is out of the range; under `try` or `catch` it fails with a `SizeError`, as above.

*Not built yet:* **Fixed places: `Decimal<digits: 10, places: 2>`.** At most 10 digits in all, 2 of them after the point, as SQL's `NUMERIC(10, 2)`: from −99,999,999.99 to 99,999,999.99. `places` is from 0 to `digits`. A plain `Decimal` keeps any number of digits, as above.

```kavach
const price: Decimal<digits: 10, places: 2> = 19.99
const tax = price * 0.18                                // 3.5982, rounded to 2 places: 3.60
const bad: Decimal<digits: 10, places: 2> = 1.234       // compile error: 3 places, the type has 2
```

- *Not built yet:* A literal with more places or digits than the type has is a compile error: it is never rounded without a word.
- *Not built yet:* Arithmetic gives the same type. A result with more places is rounded to `places`, a half away from zero, as `d.round(places)` does (14.2). A result with more digits before the point than `digits - places` panics, as overflow does (5.6).
- *Not built yet:* `Decimal<digits: 10, places: 2>(x)` converts, rounding the same way, and panics when `x` doesn't fit.
