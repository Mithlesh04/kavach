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

- `T: Number` is met by every number type: the whole-number types, `Float`, `Float32` and `Decimal`. Code may use `+ - * / %`, unary `-`, the comparisons, number literals, and `T(x)`, which converts any number to `T` as a conversion to a number type does (2.1).
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
