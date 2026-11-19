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
| `Pow` | `a ** b` | `fn pow(self, other: R) -> Out` |
| `Neg` | `-a` | `fn neg(self) -> Out` |
| `MatMul` | `a @ b` | `fn matmul(self, other: R) -> Out`, the matrix product (14.33) |
| `Comparable` | `a < b`, `a <= b`, `a > b`, `a >= b` | `fn compare(self, other: Self) -> Int`: below 0 when `self` comes first, 0 when neither does, above 0 when `other` does |
| `Compare<R, Out>` | `a < b`, `a <= b`, `a > b`, `a >= b` | `fn lt(self, other: R) -> Out`, and `le`, `gt`, `ge`: each comparison on its own, giving any type, such as a mask of `Bool`s (14.33) |

- **The other operand.** `Add` alone takes the same type: `fn add(self, other: Self) -> Self`. `Mul<Float>` takes a `Float` instead, and gives `Self`. `Mul<Meters, Area>` takes `Meters` and gives an `Area`.
- **One interface, several right operands.** A type may implement an operator's interface more than once, each time for another type on the right: `Mul<Vec2>` and `Mul<Float>`. The right operand's type chooses the method, while compiling. The methods share their name, `mul`; this is the one place a name may be written twice in a type (no overloading elsewhere, 1.4). Two for the same right operand are a compile error.

  ```kavach
  type Vec2: Mul<Vec2>, Mul<Float> {
      x: Float
      y: Float

      fn mul(self, other: Vec2) -> Vec2 { Vec2(x: self.x * other.x, y: self.y * other.y) }
      fn mul(self, other: Float) -> Vec2 { Vec2(x: self.x * other, y: self.y * other) }
  }

  const v = Vec2(x: 1.0, y: 2.0)
  v * v          // Vec2(x: 1.0, y: 4.0)
  v * 3.0        // Vec2(x: 3.0, y: 6.0)
  const n: Int = 3
  v * n          // error: `Vec2` has `*` for a `Vec2` or a `Float` on the right, and this is an `Int`
  v * 3          // a `Float`: a literal takes its type from where it goes (1.7)
  ```

- **Your type on the left.** `a * 2.0` calls `a.mul(2.0)`; `2.0 * a` is a compile error that says to write `a * 2.0`.
- **`x op= v`** is `x = x op v`, as for numbers (1.8), so `c += a` needs `Add` and a `let`.
- **Operator methods are pure:** no `uses` (8.1) and no `!` (5). `a + b` never touches the outside world and never fails, except by a panic, as `Int` overflow does. A method with either is a compile error.
- **`==` and `!=` are not interfaces.** They compare contents for every type (1.8), so `a == b` and `compare` can disagree only if the type's author makes them.
- **`Comparable` or `Compare`.** `Comparable` gives an order; `Compare` gives each of `< <= > >=` its own method and result, as a tensor's item-by-item comparisons do (`t > 0.5` is a mask, 14.33). A type implements one of the two; both is a compile error, as `<` would have two meanings. `Compare` may be implemented several times, as above.
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

**Indexing and slicing.** `x[k]`, `x[k] = v` and `x[start:end:step]` (2.19) come through interfaces too ([decision 0134](../../decisions/0134-slicing-and-indexing.md)).

| Interface | Written | The method |
|---|---|---|
| `Index<Key, Out>` | `x[k]` | `fn index(self, key: Key) -> Out` |
| `IndexSet<Key, Value>` | `x[k] = v` | `fn index_set(self, key: Key, value: Value) -> Self`: `x[k] = v` is `x = x.index_set(k, v)` |
| `Slice` | `x[start:end:step]` | `fn slice(self, part: Part) -> Self` |
| `Slice<Key>` | `x[a:b, c:d]` | `fn slice(self, part: Key) -> Self`, where `Key` is a tuple of `Part`s |

```kavach
type Row: Index<Int, Float>, IndexSet<Int, Float>, Slice {
    cells: [Float]

    fn len(self) -> Int { self.cells.len() }
    fn index(self, key: Int) -> Float { self.cells[key] }

    fn index_set(self, key: Int, value: Float) -> Row {
        let cells = self.cells
        cells[key] = value
        Row(cells: cells)
    }

    fn slice(self, part: Part) -> Row {
        const (start, end, step) = part.bounds(self.cells.len())
        let out: [Float] = []
        let i = start
        while (step > 0 and i < end) or (step < 0 and i > end) {
            out.push(self.cells[i])
            i += step
        }
        Row(cells: out)
    }
}

let r = Row(cells: [1.0, 2.0, 3.0])
print(r[#1])              // 3.0: r.index(r.len() - 1)
r[0] = 9.0                // r = r.index_set(0, 9.0)
print(r[::-1].cells)      // [3.0, 2.0, 9.0]
```

- **`Part`** is a record of `std.collections` (14.9), written in Kavach, which every file sees as it sees `Json`. The compiler makes one from what is written between the brackets: `start`, `end` and `step` are `Int?`, `none` where nothing is written; `start_from_end` and `end_from_end` are `Bool`s, `true` for an end written with `#`; `single` is `true` for a place written as one number, in a key of any length (below). `r[#2:]` gives `Part(start: 2, end: none, step: none, start_from_end: true, end_from_end: false)`.
- **`part.bounds(len) -> (Int, Int, Int)`** gives `start`, `end` and `step` as plain numbers for a value of length `len`, as Python's `slice.indices` does: `#k` becomes `len - k`, a missing step is 1, the missing ends are filled in, and both ends are clamped (to `0..=len`, or to `-1..=len - 1` for a negative step, where `-1` means before the first item). The slice holds `start`, `start + step`, and so on, while before `end`. A step of 0 breaks its `require`: a panic. `len` below 0 does too. `Part(start: none, end: none, step: -1, start_from_end: false, end_from_end: false).bounds(3)` is `(2, -1, -1)`.
- **`#k` in an index** needs a method `fn len(self) -> Int` on the type: `x[#k]` is `x.index(x.len() - k)`, as C# does for `^k`. Without `len`, `#` is a compile error that says so.
- **Many dimensions give a tuple key.** `g[1, 2]` is `g.index((1, 2))`, for `Index<(Int, Int), Out>`. When every part is a slice, `g[0:2, 1:]` is `g.slice((first, second))`, two `Part`s, for `Slice<(Part, Part)>`. A key that mixes numbers and slices goes to `index`, with a `Part` in each sliced place: `g[1, 2:]` needs `Index<(Int, Part), Out>`.
- **Keys of any length.** A type with any number of dimensions, as a tensor (14.33), is indexed with a list of `Part`s: `Index<[Part], Out>`, `IndexSet<[Part], Value>` and `Slice<[Part]>`. Each place between the commas becomes one `Part`: a slice as written, a number `i` as `Part(start: i, single: true)`, and `#k` as `Part(start: k, start_from_end: true, single: true)`, so `#k` counts from the end of its own dimension. A key of numbers only calls `index`, or `index_set` for `x[…] = v`; a key with a slice calls `slice`. One number alone is a list of one `Part`. So `t[1, #1]` is `t.index([Part(start: 1, single: true), Part(start: 1, start_from_end: true, single: true)])`.
- **Pure, as operators are:** no `uses` and no `!`. An index out of range may panic, as a list's does.
- **One implementation for each key type** ([decision 0136](../../decisions/0136-tensor-the-one-array-type.md)): a type may implement `Index` several times, once for each type of key, and the key's type picks the method when the program is compiled. A grid can have `Index<(Int, Int), T>` for `g[1, 2]` and `Index<(Int, Part), [T]>` for `g[1, :]`. Two for the same key type are a compile error.
- `x[k] = v` needs `IndexSet` and a `let`, as `x += v` needs `Add` and a `let`.
- Built-in types keep their own indexing and slicing (2.19); `extend [T]: Index` is a compile error.
- In a `gpu fn`, `x[k]` calls `index` when that method fits a `gpu fn` (15.4).

## Open questions

- `sort()`, `min()` and `max()` of a list of a `Comparable` type, and a `Comparable` type as a dict key's order. Until then, `sort_by(m => m.cents)` sorts by a key.
- Whether the number types meet `Pow`, so generic code may use `**` on a `T: Pow`.
- Bit operators (`& | ^ << >> ~`) for a program's own types (bit vectors, decision 0041's `BigInt` already has them built in).
- `#k` inside a tuple key (`g[#1, 0]`): it needs the length of each dimension, which `len` doesn't give. A compile error until decided; a list key (above) has it, as each `Part` is counted against its own dimension.
- Bounds such as `T: Index<Int, Float>` in generic code, as operators have them (decision 0115).
