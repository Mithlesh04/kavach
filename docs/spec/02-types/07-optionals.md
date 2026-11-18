# 2.7 Optionals

There is no `null`. A value that may be missing has an optional type `T?`.

```kavach
fn find_user(id: Int) -> User? { … }

const user = find_user(7)                      // User?
const name = user?.name ?? "guest"             // ?. reads through, ?? gives a default

if let u = find_user(7) {
    print(`Found {u.name}`)                  // u is a User here, not a User?
}
```

- A `T` can be used wherever a `T?` is expected.
- `none` is the missing value.
- Optionals don't nest: a `T??` is a `T?`. So `e?.time`, where the field is `time: Int?`, is an `Int?`, `none` when `e` is `none` or its `time` is.
- `?.` calls a method too: `x?.m(a)` is `none` when `x` is, else `x`'s value's `m(a)`, as an optional. It is `if let v = x { v.m(a) } else { none }`, so the arguments aren't worked out when `x` is `none`. A chain reads into nested dicts:

```kavach
const deep: {String: {String: Int}} = {"a": {"b": 7}}
print(deep.get("a")?.get("b"))          // 7
print(deep.get("x")?.get("b") ?? 0)     // 0
const word: String? = none
print(word?.upper() ?? "nothing")       // nothing
```

- On its own line, `x?.m(a)` calls `m` when `x` holds a value and does nothing when it is `none`, as `logger?.say("hi")`. A method that changes its receiver (`push`, `pop`, `add`, `remove`) can't be called through `?.`: it would change the copy `v`, not the value inside, so it is a compile error that says to test first and change the value itself, as `if d.contains_key(k) { d[k].push(x) }` (2.2).
- An `if` or `match` whose one branch is `none` and other a `T` gives a `T?`: `if found { 5 } else { none }` is an `Int?`.
- `.unwrap()` returns the value or panics. Outside `test` blocks and `example` clauses it gets a warning, as the panic says nothing of why: handle `none` with `if let`, `match` or `?? default`, or say why it can't happen with `?? panic("…")`. On a value the compiler knows is `none`, it is a compile error (1.8).

```kavach
const port = read_port().unwrap()                       // warning: panics on `none`, says nothing of why
const port = read_port() ?? panic("kavach.toml sets a port")  // no warning: the reason is written
```
