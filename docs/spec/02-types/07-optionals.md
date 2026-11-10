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
- `.unwrap()` returns the value or panics. Avoid it outside tests. On a value the compiler knows is `none`, it is a compile error (1.8).
