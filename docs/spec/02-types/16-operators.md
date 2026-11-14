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

## Open questions

- A generic function that asks for an operator, `fn total<T: Add>(items: [T], zero: T) -> T`, with `a + b` on two `T`s calling `add`; and whether the number types meet `Add` and the others as they are, as the types with an order (1.8) would meet `Comparable`.
- `sort()`, `min()` and `max()` of a list of a `Comparable` type, and a `Comparable` type as a map key's order. Until then, `sort_by(m => m.cents)` sorts by a key.
- Bit operators (`& | ^ << >> ~`) for a program's own types (bit vectors, decision 0041's `BigInt` already has them built in).
