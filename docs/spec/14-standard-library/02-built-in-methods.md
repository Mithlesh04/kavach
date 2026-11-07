# 14.2 Methods of built-in types

Numbers (`Int`, the sized integers, `Float`, `Decimal`):

| Method | Gives |
|---|---|
| `x.abs()` | The absolute value; for integers, overflow panics (`Int` smallest value) |
| `x.min(y)`, `x.max(y)` | The smaller or larger of the two |
| `x.clamp(lo, hi)` | `x` limited to `lo..=hi` |
| `checked_add` and the others in 5.6 | `none` on overflow |
| `x.div_floor(n)`, `x.mod(n)` | For whole numbers: division rounded down, and the remainder that goes with it, which has the sign of `n`: `(-7).div_floor(2)` is `-4`, `(-7).mod(3)` is `2`, where `-7 / 2` is `-3` and `-7 % 3` is `-1` (1.8). `n` of 0 panics, as for `/` ([decision 0051](../../decisions/0051-number-and-text-defaults-keep-their-speed.md)) |
| `d.round(places)`, `d.round_even(places)` | For a `Decimal`: rounded to `places` digits after the point. `round` rounds a half away from zero, as `math.round` does (14.5): `2.345.round(2)` is `2.35`, `-2.345.round(2)` is `-2.35`. `round_even` rounds a half to the even digit, the accountants' rule: `2.345.round_even(2)` is `2.34`, `2.355.round_even(2)` is `2.36`. A `Decimal` with fewer places is given back as it is; `places` below 0 panics |
| `x.near(y)` | For floats: whether `x` and `y` are equal but for rounding: the same, within one billionth of the larger, or within `1e-12` of each other near zero. For `Float32`, with room for single precision: within one hundred-thousandth, or `1e-6` near zero. `NaN` is near nothing |
| `x.bits()` | For a `Float`: its 64 bits in IEEE 754 form, as a `UInt64`, for binary formats (14.14) |
| `Float.from_bits(n)` | The `Float` with the IEEE 754 bits `n` (a `UInt64`) |

Text (`String`):

| Method | Gives |
|---|---|
| `len`, `is_empty`, `upper`, `lower`, `trim`, `contains`, `starts_with`, `ends_with`, `repeat`, `split` | As their names say; `len` counts characters |
| `s.replace(old, new)` | `s` with every `old` replaced |
| `s.lines()` | The lines, without line endings |
| `s.chars()` | The characters, as `[Char]` |
| `s.index_of(part)` | The character position of the first `part`, or `none` |
| `s.slice(start, end)` | Characters `start..end`; positions past the end are clamped |

Lists, in addition to 2.2:

| Method | Gives |
|---|---|
| `items.join(sep)` | For `[String]`: the items with `sep` between them |
| `items.sort()` | A sorted copy, for lists of values with an order (1.8): numbers, text, characters, `Bool` and tuples of them. `NaN` sorts after every number, here and in `sort_by`, `min` and `max`, so a list holding one still sorts |
| `items.sort_by(key)` | A copy sorted by `key(item)`, which must have an order (1.8); equal keys keep their order. A tuple sorts by several keys: `people.sort_by(p => (p.age, p.name))` |
| `items.reverse()` | A reversed copy |
| `items.find(f)` | The first item where `f` is true, or `none` |
| `items.count(f)` | How many items `f` is true for |
| `items.take(n)`, `items.skip(n)` | The first `n` items, or all but them |
| `items.index_of(x)` | The position of the first `x`, or `none` |
| `items.min()`, `items.max()` | The smallest or largest item, or `none` for an empty list |

Sets keep insertion order (2.2) and hold each value once:

```kavach
let seen = Set.of("asha", "ravi")
seen.add("asha")                  // already there: no change
print(seen.len())                 // 2
```

| Method | Gives |
|---|---|
| `Set.of(a, b, …)` | A set of the values |
| `Set.with_capacity(n)`, `Map.with_capacity(n)` | An empty set or map with room for `n` entries (2.2) |
| `add(x)`, `remove(x)` | Change the set; like `push`, they need a `let` (2.2) |
| `contains`, `len`, `is_empty` | As their names say |
| `to_list()` | The values in insertion order |

A `for` loop over a set visits its values in insertion order.
