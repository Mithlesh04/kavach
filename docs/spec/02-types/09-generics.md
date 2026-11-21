# 2.9 Generics

```kavach
fn first<T>(items: [T]) -> T? {
    items.get(0)
}

type Pair<A, B> {
    left: A
    right: B
}

fn largest<T: Comparable>(items: [T]) -> T? { … }
```

`<T: Comparable>` means "any type `T` that implements the interface `Comparable`".

A method may have type parameters of its own, like a function: `fn pick<T>(self, items: [T]) -> T`.

A lambda made inside a generic function may use its type parameters: they keep the types of the call that made the lambda, wherever the lambda is called later.

Built-in types are no exception: `Int` meets `<T: Describe>` only after `extend Int: Describe` (2.8). Any other type that doesn't say so is a compile error:

```kavach
fn show<T: Describe>(item: T) -> String { item.describe() }

show("text")        // compile error: String does not implement Describe
```

A generic type's arguments are usually worked out from its fields and from where the value goes. When a field doesn't show them, write them at the construction, or give the variable its type:

```kavach
type Box<I> {
    raw: String

    fn get(self) -> I ! JsonError {
        try json.decode<I>(self.raw)        // `I` is the type the box was made with
    }
}

const a = Box<Shout>(raw: text)             // written at the construction
const b: Box<Shout> = Box(raw: text)        // or from the variable's type
const c = Box<Shout, Int>(raw: text)        // compile error: `Box` takes 1 type argument, not 2
const d = Box(raw: text)                    // compile error: nothing says what `I` is
```

Inside the type's methods, and in an `extend<I> Box<I>` block (2.5), `I` is the type the value was made with.

## Number bounds

Two built-in bounds let generic code do arithmetic ([decision 0115](../../decisions/0115-generic-math-and-std-vec.md)):

- `T: Number` is met by every number type: the whole-number types, `Float`, `Float32` and `Decimal`. Code may use `+ - * / %`, unary `-`, the comparisons, number literals, and `T(x)`, which converts any number to `T` as a conversion to a number type does (2.1). The other way, `Float(x)` of a `T` converts it to that number type.
- `T: Floating` is met by `Float` and `Float32`. It is a `Number`, and code may also call `std.math`'s functions on a `T` (14.5): `math.sqrt(x)` of a `Float32` gives a `Float32`.

```kavach
fn total<T: Number>(items: [T]) -> T {
    let sum = T(0)
    for x in items {
        sum += x
    }
    sum
}

fn length<T: Floating>(x: T, y: T) -> T {
    math.sqrt(x * x + y * y)
}

total([1, 2, 3])                          // 6, an Int
length(Float32(3.0), Float32(4.0))        // 5.0, a Float32
```

- A literal in such code is of type `T`: `x * 2` doubles a `T`. A literal with a fraction, as `x * 0.5`, needs `T: Floating`.
- Each generic function is compiled once for each type it is called with, so `total` of `Float32`s runs as a loop written for `Float32` would.
- The bound is `Floating`, not `Float`, as `Float` is a type.
- Whole-number overflow and division by zero panic in generic code as elsewhere (5.6).

## Bounds on one method

A method of a generic type may add a bound to the type's parameter with `where`, after its result type. The method is then there only for the types that meet the bound; the type's other methods are there for all of them. A `where` bound is `Number` or `Floating` ([decision 0119](../../decisions/0119-bounds-on-one-method.md)).

```kavach
type Pair<T: Number> {
    a: T
    b: T

    fn sum(self) -> T { self.a + self.b }
    fn mean(self) -> T where T: Floating { (self.a + self.b) / 2.0 }
}

const counts: Pair<Int> = Pair(a: 1, b: 2)
counts.sum()                        // 3
Pair(a: 1.0, b: 2.0).mean()         // 1.5
counts.mean()                       // error: `mean` needs `T: Floating`, and `T` is `Int` here
```

- In the method's body the parameter has the bound, so a literal with a fraction and `std.math` may be used on a `T`.
- A literal with no type of its own becomes a `Float` there, as for a function's bound: `Pair(a: 1, b: 2).mean()` is `1.5`.
- `where` is written on the signature's line, and names the type's own parameters; a function writes its bounds in `<T: …>`. `where` is a word only there: elsewhere it may be a name.

## Numbers in types

A generic type may take numbers as well as types ([decision 0137](../../decisions/0137-numbers-in-types.md)). One rule says how: **types are positional, numbers are named**, as named arguments are in a call (3.2). The types come first, in order; then each number, with its name, in any order.

```kavach
type Grid<T, width: Int, height: Int> {
    rows: [[T]<len: width>]<len: height>

    fn at(self, x: Int, y: Int) -> T { self.rows[y][x] }
    fn cells(self) -> Int { width * height }
}

const g = Grid<Float, width: 2, height: 3>(rows: [[0.0, 1.0], [2.0, 3.0], [4.0, 5.0]])
print(g.cells())                                   // 6
const h: Grid<Float, height: 3, width: 2> = g      // the same type: the names count, not the order
const w = Grid<Float, 2, 3>(rows: g.rows)          // compile error: write `width: 2, height: 3`
```

- A number parameter is declared after the type parameters, as `name: Int`, and named in lower case, as a value is. A type argument never has a name; a number argument always has one. Either written the other way is a compile error that shows the right form.
- A number argument is a whole-number literal, a constant worked out while compiling (2.1), or a number parameter of the code around it. A number known only while the program runs is a compile error: a limit known only then is given as `List.capacity(n)` does (2.2).
- Inside the type, its methods and its `extend` blocks, a number parameter is a constant `Int`: `width * height` above.
- Every number parameter is given where the type is written, unless it has a default, as a function's parameter may: `type Buffer<T, size: Int = 64>`. A missing one is a compile error naming it. As for type arguments, the numbers may come from the variable's type: `const g: Grid<Float, width: 2, height: 3> = Grid(rows: …)`.
- The same type with other numbers is another type. A `Grid<Float, width: 2, height: 3>` given where a `Grid<Float, width: 3, height: 2>` is expected is a compile error naming both.
- A type is compiled once for each set of numbers it is used with, as for each type (above). The numbers are not stored in each value.

**Functions with number parameters.** A function or method declares them the same way. They are worked out from the arguments, as type arguments are; when they can't be, write them at the call.

```kavach
fn dot<n: Int>(a: [Float]<len: n>, b: [Float]<len: n>) -> Float {
    let sum = 0.0
    for i in 0..n {
        sum += a[i] * b[i]
    }
    sum
}

fn zeros<n: Int>() -> [Float]<len: n> {
    List<Float, len: n>.filled(0.0)     // n items, each 0.0 (2.2)
}

const u: [Float]<len: 3> = [1.0, 2.0, 3.0]
dot(u, [4.0, 5.0, 6.0])         // 32.0, with `n` 3
dot(u, [4.0, 5.0])              // compile error: `n` is 3 for `a` and 2 for `b`
const z = zeros<n: 4>()         // [0.0, 0.0, 0.0, 0.0]
```

**Built-in types with numbers.** Each is specified where its type is:

| Written | Means | Spec |
|---|---|---|
| `Int<bits: 24>`, `UInt<bits: 12>` | A whole number of exactly 24 bits, signed; of 12 bits, not signed | [2.1](01-built-in-types.md) (up to 64 bits) |
| `Int<range: 1..=65535>` | A whole number always from 1 to 65,535 | [2.1](01-built-in-types.md) |
| `Decimal<digits: 10, places: 2>` | At most 10 digits, 2 of them after the point | [2.1](01-built-in-types.md) (not built yet) |
| `[Float]<len: 3>`, `List<Float, len: 3>` | A list of exactly 3 items | [2.2](02-collections.md) |
| `[Float]<max: 16>`, `List<Float, max: 16>` | A list of at most 16 items | [2.2](02-collections.md) |
| `{String: Int}<max: 100>`, `Dict<String, Int, max: 100>` | A dict of at most 100 keys | [2.2](02-collections.md) |
| `Set<Int, max: 10>`, `Ring<Float, max: 100>` | A set of at most 10 values; a ring of the last 100 pushed | [2.2](02-collections.md) |
| `Bytes<len: 32>` | Exactly 32 bytes, as `crypto.sha256` gives | [2.13](13-bytes.md) |
| `Tensor<Float, dims: 2>`, `Tensor<Float, shape: [3, 4]>` | Exactly two dimensions; exactly 3 × 4 | [14.33](../14-standard-library/33-tensor.md) (not built yet) |
| `Simd<Float32, lanes: 8>` | 8 `Float32`s worked on at once | Comes with to-do item 14 (not built yet) |

A built-in type's numbers may be left out: `Int` is `Int<bits: 64>`, `[Int]` has no limit, and `Tensor<Float>` has any number of dimensions.

**`<…>` belongs to the type just before it.** A collection's own numbers come right after it, and an item type's after the item type:

```kavach
const a: {String: [Int]<max: 5>} = {"small": [1, 2]}    // a dict of lists of at most 5
const b: {String: Int<bits: 8>} = {"x": 100}            // a dict of 8-bit numbers
const c: [[Float]<len: 3>]<max: 100> = []               // at most 100 lists of 3
const d: [Int, max: 2] = []                             // compile error: write `[Int]<max: 2>`
```

**What is checked when.** The compiler checks every number it can see. The rest is checked when the program runs.

| While compiling | While running |
|---|---|
| A number argument's name and value, and a missing one | A sum or product of `bits` or `range` numbers that doesn't fit: a panic, as overflow is (5.6) |
| A literal that doesn't fit: `4096` as a `UInt<bits: 12>`, 3 items into `[Int]<max: 2>`, `1.234` with `places: 2` | A `push` past `max`: a panic; `is_full()` checks first (2.2) |
| A type given for the same type with other numbers | A `try` into a type with numbers (below) |
| An index written as a number past a `len`: `v[3]` of a `[Float]<len: 3>` | An index worked out while running, as for any list |
| `a @ b` of two tensors whose shapes are in their types | Tensor shapes the types leave out (14.33) (not built yet) |

**Into a type with numbers.** A value whose type has the numbers may be given where the type without them is expected: a `Bytes<len: 32>` where `Bytes` is, a `Tensor<Float, shape: [3, 4]>` where `Tensor<Float, dims: 2>` is. The other way takes `try` or `catch`: the type with its numbers, called with the value, fails with a `SizeError` when the value doesn't fit, as `try Int(text)` fails with a `ParseError` (2.1) and `try Dump(...)` with an `InvariantError` (7.3). Without `try` or `catch` it is a compile error, unless the value is a literal, which is checked while compiling.

```kavach
fn pair(xs: [Int]) -> [Int]<max: 2> ! SizeError {
    try List<Int, max: 2>(xs)
}

const p = List<Int, max: 2>([1, 2, 3].map(x => x * 2)) catch e => {
    print(e)              // a list of 3 items doesn't fit `List<Int, max: 2>`
    []
}
```

**Variables and operators.** The numbers of a collection's own type follow these rules:

- A `let` with no written type takes its value's type without the collection's own number, as it may change; a `const` keeps it, and so does a `let` whose type is written. The value keeps a `max` while running either way (2.2).
- Comparisons and arithmetic don't look at a collection's own number: `Bytes([2]) < Bytes([1, 9])` compares two `Bytes`, and `xs + ys` is a plain list (2.2).
- Inside code with a number parameter, a literal can't be checked against `len: n`, as `n` is known only for each call: it is a compile error. `List<T, len: n>.filled(v)` makes the list (2.2), and `try` or `catch` still converts one.

```kavach
let out = Bytes([1])                     // a `Bytes`: it may grow
out = out + Bytes([2, 3])
const key = Bytes([1, 2])                // a `Bytes<len: 2>`
let kept: Bytes<len: 2> = Bytes([3, 4])  // the written type keeps it
print(Bytes([2]) < Bytes([1, 9]))        // false

fn ones<n: Int>() -> [Int]<len: n> {
    [1, 1]                               // compile error: `n` is known only for each call
}
```

Number conversions are as before: `UInt<bits: 12>(x)` and `Int<range: 1..=65535>(x)` panic when `x` doesn't fit, as `Int8(300)` does (2.1). Under `try` or `catch`, they fail with a `SizeError` instead, as every conversion of a number into a whole-number type does:

```kavach
fn port_of(n: Int) -> Int<range: 1..=65535> ! SizeError {
    try Int<range: 1..=65535>(n)     // `0 doesn't fit in Int<range: 1..=65535>`
}
```

## Open questions

- **Arithmetic in number arguments:** `[T]<len: n + 1>`, `Grid<T, width: w * 2, height: h>`. Not yet; Rust has not made it stable either.
- **Other kinds of number parameters** for a program's own types: a list of numbers (a shape, as `Tensor`'s), a `Bool`, a range. Today a program's own number parameters are `Int`s, and the checker knows `std.tensor`'s shape rules itself (14.33).
- **A number left open:** `Tensor<Float32, shape: [_, 768]>`, or a program's own type with a number left out to mean "any, checked while running", as `Tensor<Float>` has. A function's number parameter covers most uses today: `fn embed<b: Int>(x: Tensor<Float32, shape: [b, 768]>)`.
- **A name for a type with numbers:** `type Port = Int<range: 1..=65535>` as an alias. Aliases exist only for anonymous records (2.14).
- **Numbers deeper inside a type:** a `[[Float]]` given where a `[[Float]<len: 3>]` is expected is not yet refused; only a collection's own numbers are compared, and an item's when both types have them.
- **`extend` blocks of types with numbers** (`extend<T, width: Int> Grid<T, width: width>`) are not built: methods of such a type go in its declaration.
