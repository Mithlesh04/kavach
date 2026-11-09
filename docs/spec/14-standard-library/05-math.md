# 14.5 std.math

| Name | Gives |
|---|---|
| `math.pi`, `math.e` | The constants, as `Float` |
| `math.sqrt(x)`, `math.pow(x, y)` | For `Float` or `Float32`, giving the same type; a `Float32` is worked out as a `Float` and rounded once |
| `math.floor(x)`, `math.ceil(x)`, `math.round(x)` | The same; `round` rounds halves away from zero |
| `math.gcd(a, b)`, `math.lcm(a, b)` | Greatest common divisor and least common multiple of `Int`s |
| `math.is_prime(n)` | Whether `n` is a prime number |
| `math.factorial(n)` | `n!` for `0 <= n <= 20` (larger values don't fit in `Int`) |
| `math.mean(xs)` | The average of a `[Float]`, or `none` for an empty list |
