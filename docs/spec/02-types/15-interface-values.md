# 2.15 Interface values

An interface name is also a type for values ([decision 0029](../../decisions/0029-interface-values.md)). A value of any type that implements the interface fits, and a method call on it finds the concrete type's method at run time:

```kavach
fn announce(sink: Describe) {
    print(sink.describe())
}

fn main() {
    announce(User(name: "Asha"))
    announce(Rect(width: 2, height: 3))
    const all: [Describe] = [User(name: "Ravi"), Rect(width: 1, height: 1)]
    for d in all {
        print(d.describe())
    }
}
```

- Only the interface's methods exist on the value. To use it as the concrete type, check it with `is`, below. A closed set of types that must all be covered is a union (2.6).
- Generics are the fast path: `fn show<T: Describe>(x: T)` compiles once per type with the call inlined (2.9). An interface value pays one indirection per call, and is for holding different types together.
- Printing an interface value prints the concrete value. Two interface values are equal when they hold the same concrete type with equal values.
- A method that can fail (`fn read(self) -> Int ! ReadError`) fails the same way through an interface value: the call takes `try` or `catch`, as a direct call does (chapter 5).
- A generic interface is a value type with its type arguments: a `Getter<Int>` holds any type that implements `Getter<Int>`, and its `get` gives an `Int`. A type that implements `Getter<String>` doesn't fit there.

**Checking a value's type: `is`** ([decision 0149](../../decisions/0149-is-checks-a-type.md); *not built yet*). `x is T` gives a `Bool`: whether `x` holds a `T`, for an interface value, an inline union (2.6) or a declared union's variant. Inside `if x is T { … }`, and in the rest of a condition joined by `and`, `x` is a `T`:

```kavach
fn area(s: Describe) -> Float {
    if s is Circle {
        return 3.14 * s.radius * s.radius      // `s` is a `Circle` here
    }
    0.0
}

const a: String | Int = load()
if a is String { print(a.upper()) } else { print(a + 1) }   // else: an `Int`
print(reflect.type_of(a) == String)                         // a type name compared with a `Type` (14.32)
print(shape is Shape.Circle)                                 // a declared union's variant
```

- Narrowing holds for a name the block doesn't assign to. For an inline union, `else` narrows to the members left.
- A check the compiler can already answer is a warning: `b is Name` where `b` is a `Name` is always true.
- `is` is a keyword only between a value and a type (1.6).
