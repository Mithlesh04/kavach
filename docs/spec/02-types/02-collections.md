# 2.2 Collections

Type | Shorthand | Literal |
|---|---|---|
| `List<T>` | `[T]` | `[1, 2, 3]` |
| `Dict<K, V>` | `{K: V}` | `{"asha": 31, "ravi": 27}`, empty: `{:}` |
| `Set<T>` | – | `Set.of(1, 2, 3)` |

```kavach
const names = ["Asha", "Ravi"]
const first = names[0]           // panics if out of range
const maybe = names.get(5)       // returns String? (none if out of range)

const ages = {"asha": 31}
const age = ages["asha"]         // returns Int? (dicts always return optionals)

const user = {name: "Asha", city: "Pune"}    // keys written as names are text: "name", "city"
const key = "role"
const roles = {[key]: "admin"}               // a key in brackets is the value of what is inside
```

**Dicts** are written with braces ([decision 0129](../../decisions/0129-maps-written-with-braces.md), [decision 0130](../../decisions/0130-map-renamed-dict.md)): a value `{key: value, …}`, an empty one `{:}`, and the type `{K: V}` (`Dict<K, V>`); `[K: V]` is a compile error that says to write braces. A key written as a plain name is that text; a key in brackets, `{[key]: 1}`, is the value of what is inside; other keys are values (`{"first name": 1}`, `{1: "one"}`). `["a": 1]` and `[:]` are compile errors that say to write braces. `{` after `if`, `fn`, `for`, `=>` and the like still starts a block, and `{}` is an empty block.

**Limits in the type** ([decision 0137](../../decisions/0137-numbers-in-types.md), 2.9). A list, dict, set or ring may say in its type how many entries it holds **at most**, with `max`. A list may instead say how many it holds **exactly**, with `len`. The number goes in `<…>` right after the collection's type, in either spelling:

| Written | Holds |
|---|---|
| `[Int]<max: 2>`, `List<Int, max: 2>` | At most 2 items |
| `[Float]<len: 3>`, `List<Float, len: 3>` | Exactly 3 items |
| `{String: Int}<max: 100>`, `Dict<String, Int, max: 100>` | At most 100 keys |
| `Set<Int, max: 10>` | At most 10 values |
| `Ring<Float, max: 100>` | The last 100 items pushed (below) |

`<…>` belongs to the type just before it: `{String: [Int]<max: 5>}` is a dict of lists of at most 5 items each, with no limit on its keys, and `{String: Int<bits: 8>}` is a dict of 8-bit numbers (2.1). `[Int, max: 2]` and `[Float, len: 3]` are compile errors that show `[Int]<max: 2>` and `[Float]<len: 3>`: they read as lists of two items.

```kavach
let recent: [Float]<max: 3> = [0.5]        // 0.5, and room for 2 more
recent.push(1.5)
recent.push(2.5)
print(recent.is_full())                     // true
recent.push(3.5)                            // panic: the list is full (max 3)

let ages: {String: Int}<max: 2> = {"asha": 31}
ages["asha"] = 32                           // changes a key: not adding, so no panic
ages["ravi"] = 27                           // now full
let seen: Set<Int, max: 2> = Set.of(7, 7)   // one entry: 7
let a: List<max: 2> = []                    // the item type comes from the push below
a.push(1)
const three: [Int]<max: 2> = [1, 2, 3]      // compile error: 3 items, and the type holds 2
```

- Room for all the entries is made at once, so filling never stops to make room. Adding an entry to a full one is a panic; `is_full()` says whether it is full, so a program can check first.
- A literal with more entries than the limit is a compile error. Set values or dict keys known while compiling that are equal count once, as `Set.of(7, 7)` above.
- The item types may be left out and are inferred, as for `let a = []`: `List<max: 2>`, `Dict<max: 100>`, `Set<max: 10>`.
- A limit is from 0 to 4,294,967,294 (2³² − 2); another is a compile error.

**A list of fixed length**, `[Float]<len: 3>`, holds exactly 3 items, always. An item may be set in place; `push`, `pop`, `remove` and `+=` are compile errors. An index written as a number is checked while compiling.

```kavach
let v: [Float]<len: 3> = [1.0, 2.0, 3.0]
v[0] = 0.5
v.push(4.0)              // compile error: a `[Float]<len: 3>` always holds 3 items
print(v[3])              // compile error: index 3 is past a list of 3
```

**Making a plain collection limited.** Only `try` (or `catch`) does it. The type with its limit, called with the collection, fails with a `SizeError` when it has too many entries (for `len`, not exactly that many), as `try Int(text)` fails (2.1). Without `try` or `catch` it is a compile error, unless the collection is a literal. The other way needs nothing: a limited collection may be given where a plain one is expected, and it keeps its limit, as one made by `capacity` does (below).

```kavach
fn pair(xs: [Int]) -> [Int]<max: 2> ! SizeError {
    try List<Int, max: 2>(xs)        // fails when xs has more than 2 items
}
```

**A limit known only while running.** `List.capacity(n)`, `Dict.capacity(n)`, `Set.capacity(n)` and `Ring.capacity(n)` make an empty collection of at most `n` entries, for an `n` worked out while the program runs. Its type is the plain one (`[T]`), and the limit belongs to the value. A number known while compiling is a compile error that names the type to write: `List.capacity(5)` says `List<max: 5>`. A number below 0 counts as 0; one above 4,294,967,294 is a panic. A second argument gives the first entries: a list for `List`, `Set` and `Ring`, a dict for `Dict`. More first entries than `n` is a panic, except for a ring (below).

```kavach
fn window(n: Int) -> [Float] {
    List.capacity(n, [0.5])                  // 0.5, and room for n - 1 more
}

let r: [Int] = List.capacity(5)              // compile error: the limit is known: write `List<max: 5>`
```

The items can be of any type, collections with a limit too: each inner collection keeps its own limit. Items of different kinds go in one collection through a union ([2.6](06-unions.md)):

```kavach
type Reading = Heat(celsius: Float) | Note(text: String)

let log: [Reading]<max: 100> = []
log.push(Reading.Heat(celsius: 21.5))
log.push(Reading.Note(text: "door opened"))

let groups: {String: [Int]<max: 3>}<max: 10> = {:}
groups["small"] = [1, 2]                     // a dict of at most 10 lists, each of at most 3
```

A limit goes with the collection: a copy has it too (`let b = a`), and so does the collection after `push`, `+=`, `add`, `remove`, `pop` or an item set in place. A new collection made from it (`xs + ys`, `sort()`, `map`, `filter`, `keys()`, …) has no limit, and its type is the plain one. Tuples need none: their size is fixed by their type (2.3).

Dicts and sets keep **insertion order**: iterating visits items in the order they were first added, so output and tests are always repeatable.

Methods of lists: `len`, `is_empty`, `is_full`, `get`, `first`, `last`, `contains`, `enumerate`, `push`, and `filter`, `map`, `flat_map`, `any`, `all`, `sum` with a lambda (an empty list's `sum` is the zero of the lambda's result: `0`, or `0.0` for a `Float`). Methods of dicts: `len`, `is_empty`, `is_full`, `get`, `contains_key`, `keys`, `values`, `remove`. Sets have `any`, `all`, `count`, `sum`, `find`, `for_each`, `map` and `filter` too, over their values in insertion order. More methods, and sets', are in [14.2](../14-standard-library/02-built-in-methods.md#142-methods-of-built-in-types).

There is one way to set an item: `items[i] = v` changes an item of a list, and `ages[name] = v` adds or changes a key. `push` and `remove` change the collection they are called on, so like assignment they need a `let`:

```kavach
let ages = {"asha": 31}
ages["ravi"] = 27              // adds "ravi"
ages.remove("asha")            // ages is now {"ravi": 27}

const fixed = [1, 2]
fixed.push(3)                  // error: `fixed` is a `const`: declare it with `let`
```

A changing method (`push`, `pop`, `add`, `remove`) also works on an item or a dict entry, and changes it where it is, so `rows[i].push(x)` and `groups["small"].push(x)` change the list inside. On a dict entry it means what `groups["small"] += [x]` means, and a missing key panics, naming it, as an index out of range does for a list. Any other use of `groups["small"]` reads it, and a read is an optional (`none` when the key is missing), so a method that only reads goes through `?.` or `??`:

```kavach
let groups = {"small": [1, 2]}
groups["small"].push(3)                  // {"small": [1, 2, 3]}
groups["large"].push(9)                  // panic: the dict has no key `large`
print(groups["small"]?.len())            // 3
print((groups["large"] ?? []).len())     // 0
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

**Rings.** `Ring<T, max: n>` keeps the last `n` items pushed. `Ring<T, max: n>()`, or `Ring()` where the variable's type says it, makes an empty one; when it is full, `push` drops the oldest item to make room, where a full list would panic. The items are in the order they were pushed, oldest first: `first()` is the oldest, `last()` the newest, `get(0)` the oldest, and `for` visits them in that order. A list given, as in `Ring<T, max: n>(items)` or `Ring.capacity(n, items)`, makes the first items; given more than `n`, the ring keeps the last `n`, so making a ring never fails. `Ring.capacity(n)` makes a `Ring<T>` whose size is known only while running, as for a list.

```kavach
let recent: Ring<Float, max: 3> = Ring()
for t in [1.0, 2.0, 3.0, 4.0] {
    recent.push(t)             // the fourth push drops 1.0
}
print(recent)                  // [2.0, 3.0, 4.0]
print(recent.first() ?? 0.0)   // 2.0
const kept = Ring<String, max: 2>(["a", "b", "c"])   // "b", "c"
```

A ring has the methods `push`, `len`, `is_empty`, `is_full`, `first`, `last`, `get`, `contains` and `to_list()`, which gives a list of its items, oldest first, with no limit. A ring prints as that list. A ring is a type of its own, not a list: a function that takes a `[T]` is never given a ring, so pushing onto a list never drops an item without saying so. Like a list's limit, a ring's size goes with its copies.

**What can be a key.** Any value can be a dict's key or a set's item: numbers, text, tuples, records, unions, and lists, dicts and sets too. Kavach's values never change in place (6.1), so a key can't change while it is in a dict. Keys are compared by value, as `==` compares (1.8), with two rules for floats, so every float has one place:

- `0.0` and `-0.0` are one key.
- NaN is one key: NaN equals NaN as a key or a set item, though `==` keeps NaN equal to nothing, as IEEE 754 has it.

```kavach
let zero = 0.0
const nan = zero / zero
const seen: {Float: String} = {[nan]: "not a number", 0.0: "zero"}
print(seen[nan] ?? "missing")        // not a number
print(seen[-0.0] ?? "missing")       // zero
print(Set.of(nan, nan, 0.0, -0.0).len())   // 2
const grid: {(Int, Int): String} = {(0, 0): "start"}
```
