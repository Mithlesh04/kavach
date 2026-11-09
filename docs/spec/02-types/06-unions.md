# 2.6 Unions

A union is a value that is exactly one of several variants.

```kavach
type Status = Pending | Paid | Refunded

type Shape = Circle(radius: Float) | Rect(width: Float, height: Float)

const s = Shape.Circle(radius: 2.0)
```

- Variants may carry named fields. Like record fields (2.4), they may have defaults, which may be left out: `Circle(radius: Float = 1.0)`, then `Shape.Circle()`.
- A variant is always written with its union's name, as a value and in a `match` case alike: `Status.Paid`, `Shape.Circle(radius: 2.0)`, `FsError.NotFound(path)` ([decision 0070](../../decisions/0070-variants-are-written-with-their-type.md)). So what a name means never depends on which other unions exist. A bare variant name is a compile error that says what to write, and `kavach fix` rewrites it:

  ```kavach
  const chosen = Status.parse("Paid") ?? Pending   // error: write `Status.Pending`
  ```

- A variant may have the name of a built-in type, such as `List` or `Error`: `Kind.List` is the variant, and `List` stays the built-in type.
- Unions are inspected with `match` (chapter 4), which must cover every variant.

**Enums.** A union whose variants carry no fields is an enum:

```kavach
type Status = Pending | Paid | Refunded

for s in Status.all() {
    print(s.name())                       // Pending, Paid, Refunded
}
const chosen = Status.parse("Paid") ?? Status.Pending
```

- `Status.all()` gives every variant, in the order they are written.
- `s.name()` gives the variant's name as text (`"Paid"`). It works on any union value.
- `Status.parse(text)` gives the variant with that name, or `none`.

Each variant of an enum may have a whole number, for wire formats, files and C:

```kavach
type Code = Ok = 200 | NotFound = 404 | Failed = 500

print(Code.NotFound.value())              // 404
const c = Code.from(500) ?? Code.Failed
```

- `c.value()` gives its number; `Code.from(n)` gives the variant with number `n`, or `none`.
- Either every variant has a number or none does, and no two have the same one.
- A function or method the type declares itself with one of these names (`all`, `name`, `parse`, `value`, `from`) is used instead.
