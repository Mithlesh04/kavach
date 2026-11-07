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
