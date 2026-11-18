# 2.16 Operators for your own types

A type gets an operator by implementing its built-in interface ([decision 0109](../../decisions/0109-operators-through-interfaces.md)). So fixed-point numbers, complex numbers, intervals, units and vectors can be written in Kavach, as packages, and read like built-in numbers.

```kavach
type Vec2: Add, Neg, Mul<Float> {
    x: Float
    y: Float

    fn add(self, other: Vec2) -> Vec2 { Vec2(x: self.x + other.x, y: self.y + other.y) }
    fn neg(self) -> Vec2 { Vec2(x: -self.x, y: -self.y) }
    fn mul(self, other: Float) -> Vec2 { Vec2(x: self.x * other, y: self.y * other) }
}

const a = Vec2(x: 1.0, y: 2.0)
const b = a + a * 2.0          // Vec2(x: 3.0, y: 6.0)
let c = -b
c += a                         // `c = c + a`
```

| Interface | Operator | The method |
|---|---|---|
| `Add`, `Sub`, `Mul`, `Div`, `Rem` | `a + b`, `a - b`, `a * b`, `a / b`, `a % b` | `fn add(self, other: R) -> Out`, and `sub`, `mul`, `div`, `rem` |
| `Neg` | `-a` | `fn neg(self) -> Out` |
| `MatMul` | `a @ b` | `fn matmul(self, other: R) -> Out`, the matrix product (2.17) |
| `Comparable` | `a < b`, `a <= b`, `a > b`, `a >= b` | `fn compare(self, other: Self) -> Int`: below 0 when `self` comes first, 0 when neither does, above 0 when `other` does |

- **The other operand.** `Add` alone takes the same type: `fn add(self, other: Self) -> Self`. `Mul<Float>` takes a `Float` instead, and gives `Self`. `Mul<Meters, Area>` takes `Meters` and gives an `Area`. A type implements each interface once, since its method has one name (no overloading, 1.4): `Vec2` with `Mul<Float>` has `v * 2.0` but not `v * v`.
- **Your type on the left.** `a * 2.0` calls `a.mul(2.0)`; `2.0 * a` is a compile error that says to write `a * 2.0`.
- **`x op= v`** is `x = x op v`, as for numbers (1.8), so `c += a` needs `Add` and a `let`.
- **Operator methods are pure:** no `uses` (8.1) and no `!` (5). `a + b` never touches the outside world and never fails, except by a panic, as `Int` overflow does. A method with either is a compile error.
- **`==` and `!=` are not interfaces.** They compare contents for every type (1.8), so `a == b` and `compare` can disagree only if the type's author makes them.
- Built-in types keep their own operators; `extend Int: Add` is a compile error.

```kavach
type Money: Add, Comparable {
    cents: Int

    fn add(self, other: Money) -> Money { Money(cents: self.cents + other.cents) }
    fn compare(self, other: Money) -> Int { self.cents - other.cents }
}

print(Money(cents: 150) + Money(cents: 99) > Money(cents: 200))   // true
```

**Operators in generic code.** A type parameter with an operator's interface as its bound may use that operator ([decision 0115](../../decisions/0115-generic-math-and-std-vec.md)). The number types meet `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg` and `Comparable` as they are, and `String` meets `Comparable`, so one generic function serves numbers and a program's own types:

```kavach
fn total<T: Add>(items: [T], zero: T) -> T {
    let sum = zero
    for x in items {
        sum = sum + x
    }
    sum
}

total([1, 2, 3], 0)                                  // 6
total([Money(cents: 150), Money(cents: 99)], Money(cents: 0))   // Money(cents: 249)
```

- `a + b` on a `T: Add` calls `add`: for a number, its own `+`, so each copy of the function (2.9) runs one instruction; for a program's own type, its method.
- A number's methods are the operators: `7.add(2)` is `9`, and `7.compare(9)` is `-1`. A float's `compare` gives 0 when neither comes first, as with a `NaN`.
- A type given for a `T` with operator bounds must have them: `total(["a"], "")` is a compile error, as `String` doesn't meet `Add`.

## Open questions

- `sort()`, `min()` and `max()` of a list of a `Comparable` type, and a `Comparable` type as a dict key's order. Until then, `sort_by(m => m.cents)` sorts by a key.
- Bit operators (`& | ^ << >> ~`) for a program's own types (bit vectors, decision 0041's `BigInt` already has them built in).
