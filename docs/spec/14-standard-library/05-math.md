# 14.5 std.math

| Name | Gives |
|---|---|
| `math.pi`, `math.e` | The constants, as `Float` |
| `math.sqrt(x)`, `math.pow(x, y)` | For `Float` or `Float32`, giving the same type; a `Float32` is worked out as a `Float` and rounded once |
| `math.floor(x)`, `math.ceil(x)`, `math.round(x)` | The same; `round` rounds halves away from zero |
| `math.sin(x)`, `math.cos(x)`, `math.tan(x)` | The same, for an angle `x` in radians: `math.sin(math.pi / 2.0)` is `1.0` |
| `math.asin(x)`, `math.acos(x)`, `math.atan(x)` | The same: the angle, in radians, whose sine, cosine or tangent is `x`; `asin` and `acos` of a number past -1 to 1 is `NaN` |
| `math.atan2(y, x)` | The same: the angle, in radians from -π to π, of the point `(x, y)` from the x axis, as `atan(y / x)` in the right quarter |
| `math.exp(x)`, `math.ln(x)` | The same: `e` to the power `x`, and the natural logarithm; `ln` of 0 is minus infinity, of a negative number `NaN` |
| `math.log10(x)` | The same: the logarithm to base 10; `log10(1000.0)` is `3.0` |
| `math.cbrt(x)` | The same: the cube root, of a negative number too: `cbrt(-8.0)` is `-2.0` |
| `math.hypot(x, y)` | The same: `sqrt(x * x + y * y)`, the length of the vector `(x, y)`, without the squares overflowing or vanishing on the way |
| `math.remainder(x, y)` | The same: IEEE's remainder, `x - n * y` with `n` the whole number nearest `x / y` (an even one at a tie), so it lies between `-y / 2` and `y / 2`; unlike `%`, which keeps `x`'s sign |
| `math.erfc(x)` | The same: the complementary error function, `1 - erf(x)`, exact even where `erf(x)` is near 1 |
| `math.lgamma(x)` | The same: the natural logarithm of the absolute value of the gamma function, so `lgamma(n + 1.0)` is `ln(n!)` without `n!` overflowing |
| `math.gcd(a, b)`, `math.lcm(a, b)` | Greatest common divisor and least common multiple of `Int`s |
| `math.is_prime(n)` | Whether `n` is a prime number |
| `math.factorial(n)` | `n!` for `0 <= n <= 20` (larger values don't fit in `Int`) |

The average of a list is a list method, `xs.mean(x => x)` (14.2); `math.mean(xs)` is a compile error that names it ([decision 0135](../../decisions/0135-tables-and-grouping.md)).

`sqrt`, `floor`, `ceil` and `round` are exact. The others (`pow`, `sin` to `atan2`, `exp`, `ln`, `log10` to `lgamma`) are Kavach's own code, the same on every system, interpreted or native, so each gives the same bits everywhere (2.1, [decision 0122](../../decisions/0122-floats-give-the-same-bits-everywhere.md)); each is within one unit in the last place of the exact result.
