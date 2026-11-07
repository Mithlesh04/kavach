# 14.7 std.test

`assert` (11.8) is part of the language. `std.test` has what generated tests need from a program:

- `Sample`, the interface of a type that makes its own generated values (11.3): `fn sample(gen: Gen) -> Self`.
- `Gen`, the random choices `sample` makes them from. `kavach test` seeds them, so a run can be repeated (11.9).

| Method | Gives |
|---|---|
| `gen.int(from, to)` | An `Int` from `from` to `to`, both included |
| `gen.bool()` | `true` or `false` |
| `gen.pick(items)` | One of the items; the list must not be empty |
| `gen.word()` | A word of 1 to 8 lowercase letters |

```kavach
type Point: Sample {
    x: Int
    y: Int

    fn sample(gen: Gen) -> Point {
        Point(x: gen.int(-10, 10), y: gen.int(-10, 10))
    }
}
```

Like every standard type, `Sample` and `Gen` need no `use`.
