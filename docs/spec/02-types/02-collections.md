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

`Map.with_capacity(n)` and `Set.with_capacity(n)` make an empty map or set with room for `n` entries, so filling it up to `n` never stops to make room: `const ages: [String: Int] = Map.with_capacity(10_000)`. The number is a hint, not a limit: more entries still fit, and a number below 0 counts as 0.

Maps and sets keep **insertion order**: iterating visits items in the order they were first added, so output and tests are always repeatable.

Methods of lists: `len`, `is_empty`, `get`, `first`, `last`, `contains`, `enumerate`, `push`, and `filter`, `map`, `flat_map`, `any`, `all`, `sum` with a lambda. Methods of maps: `len`, `is_empty`, `get`, `contains_key`, `keys`, `values`, `remove`. More methods, and sets, are in [14.2](../14-standard-library/02-built-in-methods.md#142-methods-of-built-in-types).

There is one way to set an item: `items[i] = v` changes an item of a list, and `ages[name] = v` adds or changes a key. `push` and `remove` change the collection they are called on, so like assignment they need a `let`:

```kavach
let ages = ["asha": 31]
ages["ravi"] = 27              // adds "ravi"
ages.remove("asha")            // ages is now ["ravi": 27]

const fixed = [1, 2]
fixed.push(3)                  // error: `fixed` is a `const`: declare it with `let`
```
