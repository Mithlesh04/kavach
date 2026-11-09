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

- Only the interface's methods exist on the value; there is no cast back to the concrete type. A closed set of types that must be told apart is a union (2.6).
- Generics are the fast path: `fn show<T: Describe>(x: T)` compiles once per type with the call inlined (2.9). An interface value pays one indirection per call, and is for holding different types together.
- Printing an interface value prints the concrete value. Two interface values are equal when they hold the same concrete type with equal values.
- A method that can fail (`fn read(self) -> Int ! ReadError`) fails the same way through an interface value: the call takes `try` or `catch`, as a direct call does (chapter 5).
- A generic interface is a value type with its type arguments: a `Getter<Int>` holds any type that implements `Getter<Int>`, and its `get` gives an `Int`. A type that implements `Getter<String>` doesn't fit there.
