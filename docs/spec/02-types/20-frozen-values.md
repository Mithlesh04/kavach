# 2.20 Frozen values

`x.freeze()` gives a `Frozen<T>`: a value whose fields and items never change, even in a `let`. `f.unfreeze()` gives a copy that can change. It works for every type: lists, dicts, records, and types a library didn't design ([decision 0141](../../decisions/0141-freeze.md)).

```kavach
let limits = {"cpu": 2, "memory": 512}.freeze()    // a Frozen<{String: Int}>
print(limits["cpu"] ?? 0)                         // 2: reading works as on the dict
limits["cpu"] = 4                                 // compile error: `limits` is frozen
let mine = limits.unfreeze()                      // a {String: Int} that can change
mine["cpu"] = 4
```

Values are already copies (6.1): no other code changes the value a variable holds. Freezing adds that the code holding the value can't change it either, so a library can hand out a value and know it stays as it was made. `pub(read)` fields (10.1) do the same for one field of a type the library declares.

## Reading and changing

Reading a frozen value works as on a `T`: its fields and items, the methods that don't change it, `for`, `print`, text, comparisons and `==`. A part read out is a copy of the part, of its own type (6.1).

Every change is a compile error that says to call `unfreeze()`: assigning a field or an item, `push` and the other methods that change a value (2.2), `+=` and the other `op=`, and passing it, or a part of it, to an `inout` parameter (3.9).

```kavach
type Order {
    id: Int
    lines: [String]
}

let order = Order(id: 7, lines: ["tea"]).freeze()
print(order.lines.len())        // 1
order.lines.push("cake")        // compile error: `order` is frozen, so it can't be changed with `push`: call `order.unfreeze()` for a copy that can change
order.id = 8                    // compile error, as above
let copy = order.unfreeze()
copy.lines.push("cake")         // fine: `copy` is a plain `Order`
```

- A `let` holding a frozen value may be given another frozen value of its type: `order = other.freeze()`. What the value holds stays fixed; the variable is still a variable.
- `f with { x: 1 }` gives a new value that isn't frozen (2.4), as `with` makes a new value.
- A plain `T` doesn't go where a `Frozen<T>` is written: call `.freeze()`, so the code shows where a value becomes frozen.
- Freezing a value that is already frozen, and unfreezing one that isn't, are compile errors.

## Nested values

`freeze()` freezes the whole value: every list, dict and record inside it. `freeze(nested: false)` freezes only its top level: its own fields and items can't be assigned, and nothing can be added to it or taken from it, but the values inside them may still change in place. `nested:` is written `true` or `false`, as it is part of the value's type. `unfreeze()` gives a copy that can change all the way down.

```kavach
let deep = [[0, 0], [0, 0]].freeze()
deep[0][1] = 5                  // compile error: `deep` is frozen, all the way down
let grid = [[0, 0], [0, 0]].freeze(nested: false)
grid[0][1] = 5                  // fine: the rows inside may change
grid[0] = [1, 1]                // compile error: `grid`'s own items are frozen
grid.push([1, 1])               // compile error: as above
```

A `Frozen<T>` written in a signature is frozen all the way down; one frozen only at its top level fits there too.

## Passing a frozen value

A `Frozen<T>` goes where a `T` is written only as an argument, and only to a function that **only reads** that parameter. The compiler knows which functions do. So a reading function stays plain, `fn total(o: Order) -> Int`, and still takes a frozen order, while no function can hand back a copy that can change.

A function only reads a parameter when every use of its name in the body reads from it:

- as the value before `.field`, `.0`, `[i]` or a slice;
- as the value a method that doesn't change it is called on, a built-in one or, when that method only reads `self`, one of the type's own;
- in `for`, an operator, a comparison, text, `with`, or a `match` whose arms don't take the whole value as a name;
- given to `print` and the other built-in functions, or as an argument to another function that only reads that parameter.

Anything else, such as putting it in a `let` or `const`, a record, a list or a field, returning it, making it the function's last line or a lambda's, or passing it to an `inout` parameter, is not only reading: the function then has a copy that could change, or could give one back. A frozen value given to it is a compile error that says why, and how to fix it: call `unfreeze()`, or write `Frozen<T>` for the parameter.

```kavach
fn total(o: Order) -> Int {
    o.lines.len() * 10                  // only reads `o`
}

fn keep(o: Order) -> Order {
    o                                   // gives `o` back
}

const order = Order(id: 7, lines: ["tea"]).freeze()
print(total(order))                     // 10
keep(order)                             // compile error: `keep` gives `o` back, so a frozen `Order` can't go there: pass `order.unfreeze()`, or write `o: Frozen<Order>`
```

Anywhere else, such as a `let` or a field written as `T`, a frozen value is a compile error that says to call `unfreeze()`. Where a type is left to be worked out, as in `let copy = order` or a generic `fn first<T>(items: [T]) -> T?` given a frozen list's items, the type worked out is the frozen one, so nothing is lost.

## Sealed with a key

`x.freeze(key: k)` seals the value: only `f.unfreeze(key: k)`, with a key equal to `k`, opens it. The key's type is part of the frozen value's type, written second: `Frozen<T, K>`. A library makes the key a value of a type of its own that no other code can make, so only the library unfreezes what it froze.

```kavach
// session.kav
type SessionKey {
    n: Int
}

const key = SessionKey(n: 1)

pub type Session {
    pub user: String
    pub roles: [String]
}

pub fn open(user: String) -> Frozen<Session, SessionKey> {
    Session(user: user, roles: ["reader"]).freeze(key: key)
}

pub fn promote(s: Frozen<Session, SessionKey>) -> Frozen<Session, SessionKey> {
    let changed = s.unfreeze(key: key)
    changed.roles.push("admin")
    changed.freeze(key: key)
}
```

```kavach
// main.kav
use session

fn main() {
    const s = session.open("asha")
    print(s.roles)                       // ["reader"]: other modules read it
    const t = session.promote(s)
    s.unfreeze()                         // compile error: `s` is sealed with a key: only `unfreeze(key: …)` opens it
}
```

A wrong key is caught two ways:

- **While compiling, by its type.** The key given to `unfreeze` must be of the type the value was sealed with. When that type is private to its module, only that module can name it, so code anywhere else that calls `unfreeze(key: …)` on the value is a compile error that names the module. A type that is `pub` but has a private field can't be made outside its module either (10.1).
- **While running, by its value.** A key of the right type that isn't equal (`==`, 1.8) to the one the value was sealed with is a panic that says the key is wrong (5.6). `unfreeze()` without a key on a sealed value, and `unfreeze(key: …)` on a value with no key, are compile errors.

A `match` on a sealed value reads its parts in its arms, or names the whole value in every arm, which keeps it sealed. Both in one `match` is a compile error, as the name would hold the value without its key: name it in every arm, or unfreeze it first.

A sealed value carries its key while the program runs; a value frozen without a key is the plain value, and freezing and unfreezing it costs nothing.

## Open questions

- **A part read out of a deep-frozen value** is a plain copy today. Should `order.lines` give a `Frozen<[String]>`, as nested objects stay frozen in JavaScript?
- **Printing and JSON.** A frozen value prints and encodes as its `T`. Should it say it is frozen, as a debugger might?
- **`T` into `Frozen<T>`.** Should a plain value be frozen where a `Frozen<T>` is written, without `.freeze()`?
- **A wrong key at run time** panics. Should `unfreeze(key: …)` instead fail with an error a program handles (5.1)?
- **Sealed values in generic code.** Code generic over `T`, given a sealed value, holds it with its key; it can't read the value's fields anyway, but `print` and `==` there see the key too.
