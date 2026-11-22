# 2.21 Distinct types

`type NetId = distinct Int` makes a new type that holds an `Int` but is never mixed with an `Int` or with another distinct type. Ids of different things can't be swapped by mistake, and they cost nothing: a `NetId` is stored and passed as the `Int` it holds.

```kavach
type NetId = distinct Int
type CellId = distinct Int

fn driver(net: NetId) -> CellId {
    CellId(net.value * 2)
}

const n = NetId(7)
const c = driver(n)             // a CellId
print(c.value)                  // 14
driver(c)                       // compile error: driver takes a NetId, not a CellId
driver(7)                       // compile error: an Int is not a NetId
```

- **What it can hold:** a built-in number type (`Int`, `Float`, the sized and range numbers, `Decimal`, `BigInt`), `Bool`, `Char` or `String`. `type Name = distinct T` with another `T` is a compile error.
- **Making one:** `NetId(7)` takes the value it holds, of exactly that type. **Reading it back:** `n.value`.
- **What it does:** `==` and `!=` between two values of the same distinct type, and `<`, `<=`, `>` and `>=` when the value it holds can be ordered. Nothing else: no arithmetic and none of the held type's methods. Work on `n.value` and make a new one: `NetId(n.value + 1)`.
- **Where it goes:** anywhere a type goes: lists, dicts and sets (as keys too), optionals, records and generics. A list of them sorts by the values they hold.
- **Printing and JSON:** it prints, fills a template and is written to and read from JSON (2.12) as the value it holds: `print(n)` prints `7`.
- **A `let` can change it** as a whole, `n = NetId(8)`, or through its value, `n.value += 1`.

```kavach
type Seat = distinct Int

let seats: {Seat: String} = {:}
seats[Seat(3)] = "Asha"
let s = Seat(3)
s.value += 1
print(seats.get(Seat(3)) ?? "free")    // Asha
print(s)                               // 4
```

A one-field record (`type NetId { id: Int }`, 2.4) is distinct too, and can have methods and invariants, but it is a record: native code makes it as an object of its own.

A distinct type has no methods, interfaces or invariants of its own yet (2's open questions).
