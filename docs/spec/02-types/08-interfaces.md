# 2.8 Interfaces

An interface is a set of methods that a type promises to have.

```kavach
interface Describe {
    fn describe(self) -> String
}

type User: Describe {
    name: String

    fn describe(self) -> String { `User {self.name}` }
}

extend Rect: Describe {
    fn describe(self) -> String { `Rect {self.width}x{self.height}` }
}
```

A type implements an interface only when it says so (`type User: Describe` or `extend Rect: Describe`). This holds for types from other languages too (10.7): wrap them with `extend ForeignType: Describe { … }`.

A type that says so keeps the promise, or it is a compile error. It has each of the interface's methods, and each takes as many values as the interface's, marks the same parameters `inout` (3.9), and can fail exactly when the interface's method can (chapter 5). Its parameters, its value and its error have the interface's types, read with `Self` as the implementing type and the interface's type arguments put in:

```kavach
interface Counter {
    fn bump(self, inout n: Int)
}

type Step: Counter {
    size: Int

    fn bump(self, n: Int) { }   // error: `n` must be `inout`, as in `Counter`
}
```

```kavach
interface Getter<T> {
    fn get(self) -> T
}

type Name: Getter<String> {
    text: String

    fn get(self) -> Int { 0 }   // error: `Getter<String>.get` gives a `String`
}
```

**An interface method says what it may use** ([decision 0147](../../decisions/0147-interfaces-declare-uses.md); *not built yet*: today an interface method has no `uses`, and an implementation's is not compared). It lists its capabilities with `uses`, as a function does (8.1), and one without `uses` may use only the package's default capabilities (8.1, rule 7). An implementation may use only what the interface's method lists, so code that calls the method through the interface, or through a generic bound such as `T: Describe`, needs exactly that, and knows it from the interface alone:

```kavach
interface Report {
    fn write(self) ! Error uses Fs
}

fn save(r: Report) ! Error uses Fs {   // calling `write` needs `Fs`
    try r.write()
}

type User: Describe {
    name: String

    fn describe(self) -> String uses Console {   // error: `Describe.describe` uses nothing
        print(self.name)
        self.name
    }
}
```

An interface is also a value type: see [2.15](15-interface-values.md).
