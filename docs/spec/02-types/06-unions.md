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

Each variant of an enum may carry a **value**: a whole number, for wire formats, files and C, or a text, for the names other systems use, which often aren't names a variant can have:

```kavach
type Code = Ok = 200 | NotFound = 404 | Failed = 500

print(Code.NotFound.value())              // 404
const c = Code.from(500) ?? Code.Failed
```

```kavach
type Method = Get = "GET" | Post = "POST" | Put = "PUT" | Delete = "DELETE"
type Country = India = "IN" | UnitedStates = "US" | UnitedKingdom = "GB"
type Currency = Rupee = "INR" | Dollar = "USD" | Euro = "EUR"
type Mime = Json = "application/json" | Html = "text/html"

print(Method.Post.value())                // POST
const home = Country.from("GB") ?? Country.India
print(home)                               // UnitedKingdom
```

- `c.value()` gives its value: an `Int` for numbers, a `String` for text. `Code.from(value)` gives the variant with that value, or `none`.
- Past `value()` and `from`, the value changes only JSON, which writes and reads it instead of the name (2.12). A variant still prints as its name, `home.name()` is `"UnitedKingdom"`, `Country.parse(text)` reads a name, and `match` works as on any enum.
- Either every variant has a value or none does, no two have the same one, and one type's values are all numbers or all text.
- A text value is written in quotes, exactly as it is: spaces, dots, any script (`Hindi = "हिन्दी"`), and empty text too. Text with a `"` in it is written in backticks with nothing filled in: `` Inch = `"` ``. A value is fixed when compiling, so a `{…}` to fill in is a compile error.
- A variant's name is one name, with no dots or spaces; the text other systems use goes in the value. `type Country = India | U.S` is a compile error that says to write `US = "U.S"`, and so is a variant written as text, as TypeScript writes a union, `type Lang = "en" | "hi"`, which says to write `En = "en"`.
- A function or method the type declares itself with one of these names (`all`, `name`, `parse`, `value`, `from`) is used instead.
