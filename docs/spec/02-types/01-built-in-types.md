# 2.1 Built-in types

| Type | Meaning |
|---|---|
| `Int` | 64-bit signed integer |
| `Int8`, `Int16`, `Int32`, `UInt8`, `UInt16`, `UInt32`, `UInt64` | Sized integers |
| `Float` | 64-bit floating point |
| `Float32` | 32-bit floating point: rounded to single precision at every step, and printed as the shortest text that reads back as the same `Float32` (`0.1`). What GPUs compute fastest (chapter 15) |
| `Decimal` | Exact base-10 number, with as many digits as it needs. Use it for money |
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
| `String` → a number, `Bool` or `Char` | The text read whole, with spaces around it allowed. It **can fail** with a `ParseError`, so it takes `try` or `catch`. `Int("23sd")` fails, as does `Int("300")` into `Int8`. `Bool` reads `true` or `false`; `Char` reads text of exactly one character |
| `Char` → a whole number | The character's Unicode code point: `Int('a')` is `97` |
| A whole number → `Char` | The character with that code point. One that isn't a character (a negative number, a surrogate, above `0x10FFFF`) panics |

When the text is a literal, the compiler reads it itself: `Int("2")` is the number `2` and can't fail, and `Int("abc")` is a compile error.

To find a number inside other text (`"order 23 shipped"`), use `std.text` (14.9).

**`Decimal` has no size limit** ([decision 0042](../../decisions/0042-decimal-has-no-size-limit.md)): a value of up to about 38 digits is held in a fast 128-bit form, and a bigger or more precise one keeps all its digits in a bigger form, with no change to the code. A division that doesn't end keeps 18 digits after the point.

```kavach
const rate: Decimal = 0.000000000000000000000000000001
const huge: Decimal = 99999999999999999999999999999999999999.99
print(huge + rate)        // 99999999999999999999999999999999999999.990000000000000000000000000001
```

**Constants are exact** ([decision 0043](../../decisions/0043-constants-are-exact.md)): a constant of number literals (`+ - * /`, a minus, and `**` with a whole exponent) is worked out exactly while compiling, then rounded once to its type. Values worked out while the program runs are rounded at each step, as on every machine.

```kavach
print(0.1 + 0.2)             // 0.3: one constant, rounded once
print(0.1 + 0.2 == 0.3)      // true
let a = 0.1
print(a + 0.2)               // 0.30000000000000004: worked out at run time
```

**`BigInt`** holds a whole number of any size ([decision 0041](../../decisions/0041-bigint-is-built-in.md)). It takes `+ - * / % **`, comparisons and `==`, works as a map key, and prints as its digits. `BigInt(n)` makes one from any whole number, and a literal takes the type its place expects.

A constant that doesn't fit `Int` is a `BigInt` when no type is written: a whole-number literal, or `+ - * / % **` and a minus on literals. The compiler works out its exact value. Where a type is written, a constant that doesn't fit it is a compile error. Values worked out while the program runs keep their type, so `Int` arithmetic still panics on overflow (5.6).

```kavach
let d = 234234 * 23424293429030023    // a BigInt: 5486765947055418407382
const big = 2 ** 100                  // a BigInt
const n = BigInt(7) ** 30             // BigInt(…) from an Int
print(d + 1)                          // 1 is a BigInt here
const x: Int = 2 ** 70                // error: doesn't fit in Int
```
