# 2.7 Optionals

There is no `null`. A value that may be missing has an optional type `T?`.

```kavach
fn find_user(id: Int) -> User? { … }

const user = find_user(7)                      // User?
const name = user?.name ?? "guest"             // ?. reads through, ?? gives a default

if let u = find_user(7) {
    print("Found {u.name}")                  // u is a User here, not a User?
}
```

- A `T` can be used wherever a `T?` is expected.
- `none` is the missing value.
- Optionals don't nest: a `T??` is a `T?`. So `e?.time`, where the field is `time: Int?`, is an `Int?`, `none` when `e` is `none` or its `time` is.
- `.unwrap()` returns the value or panics. Outside `test` blocks and `example` clauses it gets a warning, as the panic says nothing of why: handle `none` with `if let`, `match` or `?? default`, or say why it can't happen with `?? panic("…")`. On a value the compiler knows is `none`, it is a compile error (1.8).

```kavach
const port = read_port().unwrap()                       // warning: panics on `none`, says nothing of why
const port = read_port() ?? panic("kavach.toml sets a port")  // no warning: the reason is written
```
