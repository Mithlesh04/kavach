# 2.2 Collections

Type | Shorthand | Literal |
|---|---|---|
| `List<T>` | `[T]` | `[1, 2, 3]` |
| `Map<K, V>` | `[K: V]` | `["asha": 31, "ravi": 27]`, empty: `[:]` |
| `Set<T>` | – | `Set.of(1, 2, 3)` |

```kavach
const names = ["Asha", "Ravi"]
const first = names[0]           // panics if out of range
const maybe = names.get(5)       // returns String? (none if out of range)

const ages = ["asha": 31]
const age = ages["asha"]         // returns Int? (maps always return optionals)
```

`List.capacity(n)`, `Map.capacity(n)` and `Set.capacity(n)` make an empty list, map or set that holds **at most `n` entries**. Room for all `n` is made at once, so filling it never stops to make room, and it never grows past `n`: adding an entry to a full one is a panic. `is_full()` says whether it is full, so a program can check first. A number below 0 counts as 0; one above 4,294,967,294 (2³² − 2) is a panic.

A second argument gives the first entries: a list for `List.capacity` and `Set.capacity`, a map for `Map.capacity`. More first entries than `n` is a panic.

```kavach
let recent: [Float] = List.capacity(3, [0.5])   // 0.5, and room for 2 more
recent.push(1.5)
recent.push(2.5)
print(recent.is_full())                          // true
recent.push(3.5)                                 // panic: the list is full (capacity 3)

let ages: [String: Int] = Map.capacity(2, ["asha": 31])
ages["asha"] = 32              // changes a key: not adding, so no panic
ages["ravi"] = 27              // now full
let seen: Set<Int> = Set.capacity(2, [7, 7])     // one entry: 7
```

The items can be of any type, collections with a limit too: each inner collection keeps its own limit. Items of different kinds go in one collection through a union ([2.6](06-unions.md)):

```kavach
type Reading = Heat(celsius: Float) | Note(text: String)

let log: [Reading] = List.capacity(100)
log.push(Reading.Heat(celsius: 21.5))
log.push(Reading.Note(text: "door opened"))

let groups: [String: [Int]] = Map.capacity(10)
groups["small"] = List.capacity(3, [1, 2])     // a map of lists, each with a limit
```

The limit belongs to the value: a copy has it too (`let b = a`), and so does the collection after `push`, `+=`, `add`, `remove`, `pop` or an item set in place. A new collection made from it (`xs + ys`, `sort()`, `map`, `filter`, `keys()`, …) has no limit. Tuples need none: their size is fixed by their type (2.3).

Maps and sets keep **insertion order**: iterating visits items in the order they were first added, so output and tests are always repeatable.

Methods of lists: `len`, `is_empty`, `is_full`, `get`, `first`, `last`, `contains`, `enumerate`, `push`, and `filter`, `map`, `flat_map`, `any`, `all`, `sum` with a lambda (an empty list's `sum` is the zero of the lambda's result: `0`, or `0.0` for a `Float`). Methods of maps: `len`, `is_empty`, `is_full`, `get`, `contains_key`, `keys`, `values`, `remove`. More methods, and sets, are in [14.2](../14-standard-library/02-built-in-methods.md#142-methods-of-built-in-types).

There is one way to set an item: `items[i] = v` changes an item of a list, and `ages[name] = v` adds or changes a key. `push` and `remove` change the collection they are called on, so like assignment they need a `let`:

```kavach
let ages = ["asha": 31]
ages["ravi"] = 27              // adds "ravi"
ages.remove("asha")            // ages is now ["ravi": 27]

const fixed = [1, 2]
fixed.push(3)                  // error: `fixed` is a `const`: declare it with `let`
```

`xs + ys` is a new list: the items of `xs`, then those of `ys`. Both are lists of the same type, and neither changes. `xs += ys` adds the items of `ys` to the end of `xs`, so like `push` it needs a `let`; when nothing else holds `xs`, it grows in place (6.1), without copying what it holds:

```kavach
const evens = [2, 4]
const all = evens + [6, 8]     // [2, 4, 6, 8]; evens is still [2, 4]
let log = ["start"]
log += ["step 1", "step 2"]    // log is now ["start", "step 1", "step 2"]
```

A range (`a..b`, `a..=b`, 1.8) has the methods of a list that read it, as the list of its numbers in order would give them: `(1..=3).map(i => i * i)` is `[1, 4, 9]`, and `(0..n).sum(i => i)` adds `0` to `n - 1`. `to_list()` gives that list. Methods that change a list (`push`, `pop`) are not a range's.

```kavach
const squares = (1..=4).map(i => i * i)         // [1, 4, 9, 16]
const evens = (0..10).filter(i => i % 2 == 0)   // [0, 2, 4, 6, 8]
const total = (1..=100).sum(i => i)             // 5050
const has = (0..10).contains(10)                // false: 10 is past the end
const digits = (0..10).to_list()                // [0, 1, …, 9]
```

**Rings.** `Ring<T>` keeps the last `n` items pushed. `Ring.capacity(n)` makes an empty one; when it is full, `push` drops the oldest item to make room, where a full list would panic. The items are in the order they were pushed, oldest first: `first()` is the oldest, `last()` the newest, `get(0)` the oldest, and `for` visits them in that order. A second argument gives the first items, as for a list; given more than `n`, the ring keeps the last `n`.

```kavach
let recent: Ring<Float> = Ring.capacity(3)
for t in [1.0, 2.0, 3.0, 4.0] {
    recent.push(t)             // the fourth push drops 1.0
}
print(recent)                  // [2.0, 3.0, 4.0]
print(recent.first() ?? 0.0)   // 2.0
const kept = Ring.capacity(2, ["a", "b", "c"])   // "b", "c"
```

A ring has the methods `push`, `len`, `is_empty`, `is_full`, `first`, `last`, `get`, `contains` and `to_list()`, which gives a list of its items, oldest first, with no limit. A ring prints as that list. `Ring<T>` is a type of its own, not a list: a function that takes a `[T]` is never given a ring, so pushing onto a list never drops an item without saying so. Like a list's limit, a ring's size goes with its copies.
