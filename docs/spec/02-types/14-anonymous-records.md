# 2.14 Anonymous records

A record value or record type can be written inline, with no declaration ([decision 0028](../../decisions/0028-anonymous-records.md)):

```kavach
const h = (on: add_tax, retries: 3)
print(h.retries)                     // 3
const t = h.on(100)                    // a function field, called (3.4)

fn minmax(xs: [Int]) -> (lo: Int, hi: Int) {
    (lo: xs.min() ?? 0, hi: xs.max() ?? 0)
}
const r = minmax([3, 1, 4])
print(r.lo)                          // named fields, not r.0
```

- `(on: f, retries: 3)` builds an anonymous record of type `(on: fn(Int) -> Int, retries: Int)`. Fields are read and (through a `let`) assigned like any record's.
- The same syntax works in parameter and return types: a tuple whose fields have names.
- One named field still makes a record: `(on: f)`. `()` stays nothing, and a parenthesized expression `(x)` has no field name.
- Anonymous record types match **structurally**: same field names and types, in any order. Declared `type`s stay **nominal** (2.4) and keep what anonymity can't give: methods, `invariant`s, `secret` fields, interfaces.
- `type Handler = (on: fn(Int) -> Int)` declares an **alias**: the anonymous type under a name. Values flow freely between the alias and the written-out shape. Turning an alias into a nominal `type` is the one step where construction sites change.

Start loose, harden later: a shape can live as an anonymous record while a design settles, gain an alias when it is shared, and become a nominal `type` when it needs methods or invariants.
