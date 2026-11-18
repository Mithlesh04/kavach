# 2.5 Methods

Methods are declared inside the type body. `self` is the value the method is called on.

```kavach
type Rect {
    width: Float
    height: Float

    fn area(self) -> Float {
        self.width * self.height
    }

    /// A function without `self` is called on the type: Rect.square(2.0)
    fn square(size: Float) -> Rect {
        Rect(width: size, height: size)
    }
}
```

Use `extend` to add methods to a union, or to a type from another module:

```kavach
extend String {
    fn shout(self) -> String { self.upper() + "!" }
}
```

To extend a generic type, name its parameters after `extend`, as a generic function does: `extend<S> App<S>`. Each name stands for the type's own parameter in its place, so a type's many methods can be spread over files. A name that isn't one of the type's parameters is an error.

```kavach
// app.kav
pub type App<S> {
    pub state: S
    pub routes: Int = 0
}

// routes.kav
use app.{App}

extend<S> App<S> {
    pub fn route(self) -> App<S> {
        self with { routes: self.routes + 1 }
    }

    pub fn state_of(self) -> S {
        self.state
    }
}
```

Lists, maps, sets and rings take methods too. Name each item type after `extend`, as `extend<T> [T]`, `extend<K, V> [K: V]`, `extend<T> Set<T>` or `extend<T> Ring<T>`; the methods then work for every item type, and `T` is the item type of the one called on. A range has a list's added methods, as it has a list's own (2.2).

```kavach
extend<T> [T] {
    fn second(self) -> T? {
        if self.len() > 1 { self[1] } else { none }
    }
}

extend<K, V> [K: V] {
    fn has_all(self, keys: [K]) -> Bool {
        keys.all((k) => self.contains_key(k))
    }
}

const s = [10, 20, 30].second()                  // 20
const ok = {"a": 1, "b": 2}.has_all(["a", "z"])  // false
```

- An item type written out, as `extend [Int]`, is an error: the methods are for every list.
- A method with the name of one the collection already has (`len`, `push`, …) is an error, so `xs.len()` always means the same.
- Methods are added, never changed or taken away.
