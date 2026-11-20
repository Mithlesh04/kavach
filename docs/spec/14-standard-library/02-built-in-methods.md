# 14.2 Methods of built-in types

Numbers (`Int`, the sized integers, `Float`, `Decimal`):

| Method | Gives |
|---|---|
| `x.abs()` | The absolute value; for integers, overflow panics (`Int` smallest value) |
| `x.min(y)`, `x.max(y)` | The smaller or larger of the two; `NaN` when either is `NaN`, as IEEE 754 says |
| `x.clamp(lo, hi)` | `x` limited to `lo..=hi`; `NaN` when any of them is. `lo` above `hi` breaks its contract: a panic |
| `checked_add` and the others in 5.6 | `none` on overflow |
| `x.div_floor(n)`, `x.mod(n)` | For whole numbers: division rounded down, and the remainder that goes with it, which has the sign of `n`: `(-7).div_floor(2)` is `-4`, `(-7).mod(3)` is `2`, where `-7 / 2` is `-3` and `-7 % 3` is `-1` (1.8). `n` of 0 panics, as for `/`; the smallest `Int`'s `mod(-1)` is 0, as its `% -1` is ([decision 0051](../../decisions/0051-number-and-text-defaults-keep-their-speed.md)) |
| `d.round(places)`, `d.round_even(places)` | For a `Decimal`: rounded to `places` digits after the point. `round` rounds a half away from zero, as `math.round` does (14.5): `2.345.round(2)` is `2.35`, `-2.345.round(2)` is `-2.35`. `round_even` rounds a half to the even digit, the accountants' rule: `2.345.round_even(2)` is `2.34`, `2.355.round_even(2)` is `2.36`. A `Decimal` with fewer places is given back as it is; `places` below 0 panics |
| `x.near(y)` | For floats: whether `x` and `y` are equal but for rounding: the same, within one billionth of the larger, or within `1e-12` of each other near zero. For `Float32`, with room for single precision: within one hundred-thousandth, or `1e-6` near zero. `NaN` is near nothing, and an infinity only itself |
| `x.is_nan()`, `x.is_finite()`, `x.is_infinite()` | For floats: whether `x` is `NaN`; whether it is a number that is neither `NaN` nor an infinity; whether it is `inf` or `-inf`. `NaN` is the one value `==` never holds for, so `x == x` is `false` for it: `xs.filter(x => not x.is_nan())` leaves it out |
| `x.fract()` | For floats: `x - math.floor(x)`, the part above the whole number at or below `x`, as WGSL's `fract`, on the CPU and in `gpu fn`s alike: `2.75.fract()` is `0.75`, `(-0.25).fract()` is `0.75` |
| `x.bits()` | For a `Float`: its 64 bits in IEEE 754 form, as a `UInt64`, for binary formats (14.14) |
| `Float.from_bits(n)` | The `Float` with the IEEE 754 bits `n` (a `UInt64`) |

Text (`String`):

| Method | Gives |
|---|---|
| `len`, `is_empty`, `upper`, `lower`, `trim`, `starts_with`, `ends_with`, `repeat` | As their names say; `len` counts characters |
| `s.split(sep)` | The parts of `s` between each `sep`: `"a,b".split(",")` is `["a", "b"]`. An empty `sep` is a panic that says to use `chars()`, as Python's `split` refuses one |
| `s.contains(part)` | Whether `part` is in `s`; `part` is a `String` or a `Char`: `"aeiou".contains(c)`. An empty `part` is in every text: `"abc".contains("")` is `true` |
| `s.replace(old, new)` | `s` with every `old` replaced |
| `s.lines()` | The lines, without line endings; `"".lines()` is `[]`, as Python's `splitlines` and Rust's `lines` give |
| `s.chars()` | The characters, as `[Char]` |
| `s.index_of(part)` | The character position of the first `part`, or `none`; an empty `part` is found at once, at `0` |
| `s[i]`, `s[#k]` | Character `i`, or `k` from the end, as a `Char` (2.19); counted as `len` counts; out of range panics |
| `s[start:end:step]` | A slice (2.19): characters `start..end`, every `step`th; positions past the end are clamped. `s[:n]` is at most `n` characters, and `s[::-1]` is `s` reversed. There are no `slice` and `truncate` methods |
| `s.trim_start()`, `s.trim_end()` | `s` without the white space at its start, or at its end, as `trim` takes it off both |
| `s.last_index_of(part)` | The character position of the last `part`, or `none` |
| `s.count(part)` | How many times `part` is in `s`, not counting overlaps: `"banana".count("ana")` is `1`. An empty `part` is counted between every two characters and at both ends: `s.len() + 1` |
| `s.remove_prefix(p)`, `s.remove_suffix(p)` | `s` without `p` at its start, or at its end; `s` itself when it doesn't start or end with `p` |
| `s.capitalize()` | `s` with its first character in upper case: `"asha".capitalize()` is `"Asha"` |
| `s.title()` | Each word of `s` with its first character in upper case and the rest in lower case: `"the tALE of two".title()` is `"The Tale Of Two"`. Words are as `words()` finds them, and the spaces between them stay as they were |
| `s.words()` | The parts of `s` between runs of spaces, tabs and line breaks: `"  to be	or ".words()` is `["to", "be", "or"]` |
| `s.pad_left(width, fill: ' ')`, `s.pad_right(width, fill: ' ')` | `s` with `fill` (a `Char`) added before it, or after it, until it is `width` characters long: `"7".pad_left(3, '0')` is `"007"`. Text already that long is as it was. `width` is at most 1,000,000 |
| `s.is_blank()` | Whether `s` has only spaces, tabs and line breaks, or nothing |

Lists, in addition to 2.2:

| Method | Gives |
|---|---|
| `items.join(sep)` | For `[String]` and `[Char]`: the items with `sep` between them: `["a", "b"].join("-")` is `"a-b"` |
| `items.sort()` | A sorted copy, for lists of values with an order (1.8): numbers, text, characters, `Bool` and tuples of them. `NaN` sorts after every number, here and in `sort_by`, `min` and `max`, so a list holding one still sorts |
| `items.sort_by(key)` | A copy sorted by `key(item)`, which must have an order (1.8); equal keys keep their order. A tuple sorts by several keys: `people.sort_by(p => (p.age, p.name))` |
| `items.find(f)` | The first item where `f` is true, or `none` |
| `items.find_index(f)`, `items.find_last_index(f)` | The position of the first item where `f` is true, or of the last, or `none` |
| `items.for_each(f)` | Calls `f` on every item, in order, for what it does; gives nothing: `names.for_each(n => print(n))` |
| `items.count(f)` | How many items `f` is true for |
| `items[start:end:step]` | A slice (2.19): items `start..end`, every `step`th, clamped to the list, as for text. `xs[:n]` is the first `n` items, `xs[n:]` all but them, and `xs[::-1]` a reversed copy. There are no `slice`, `take`, `skip` and `reverse` methods (2.19) |
| `items.index_of(x)` | The position of the first `x`, or `none` |
| `items.min()`, `items.max()` | The smallest or largest item, or `none` for an empty list |
| `items.pop()` | The last item, which it takes out of the list, or `none` for an empty list. It changes the list, so like `push` it needs a `let` (2.2), and it takes the item out in place when nothing else holds the list: `const top = stack.pop() ?? 0` |

A negative count or width counts as 0, as for `List.capacity` (2.2): `"ab".repeat(-1)` is `""`, and `"ab".pad_left(-1)` is `"ab"`.

Sets keep insertion order (2.2) and hold each value once:

```kavach
let seen = Set.of("asha", "ravi")
seen.add("asha")                  // already there: no change
print(seen.len())                 // 2
```

| Method | Gives |
|---|---|
| `Set.of(a, b, …)` | A set of the values |
| `Set.capacity(n)`, `Set.capacity(n, first)` | A set of at most `n` values, maybe with the values of the list `first` (2.2) |
| `add(x)`, `remove(x)` | Change the set; like `push`, they need a `let` (2.2) |
| `contains`, `len`, `is_empty`, `is_full` | As their names say; `is_full` for a set made by `Set.capacity(n)` holding `n` values (2.2) |
| `to_list()` | The values in insertion order |
| `any(test)`, `all(test)`, `count(test)`, `sum(f)`, `find(test)`, `for_each(f)` | As for a list, over the values in insertion order: `find` gives the first that passes, or `none` |
| `map(f)`, `filter(test)` | A new set: the results, or the values that pass, in insertion order; equal results are kept once |

A `for` loop over a set visits its values in insertion order.

```kavach
const sizes = Set.of("pear", "fig", "apple")
print(sizes.all(w => w.len() >= 3))       // true
print(sizes.sum(w => w.len()))            // 12
print(sizes.map(w => w.len() % 2))        // Set.of(0, 1): equal results kept once
print(sizes.filter(w => w.contains("p"))) // Set.of("pear", "apple")
```

Each job has one method. Names other languages use for the same job are compile errors that name Kavach's: `some` names `any(test)`, `every` names `all(test)`, `includes` names `contains(x)`, and `each` and `forEach` name `for_each(f)`. Method names are `snake_case`, as all names of values are (1.5): `findIndex` and `indexOf` name `find_index(test)` and `index_of(x)`. `find_index` finds the first match, as `index_of` finds the first `x`, so `find_first_index` names `find_index(test)` too.

A ring (`Ring<T>`, 2.2) has `push`, `len`, `is_empty`, `is_full`, `first`, `last`, `get`, `contains` and `to_list`, oldest item first; a push onto a full ring drops the oldest.

### Grouping and statistics

A list of records works like a table. It can be grouped, counted and summed per key, summarized, and joined with another list on a key, as pandas, polars and R's dplyr do ([decision 0135](../../decisions/0135-tables-and-grouping.md)). Every key and every column is a lambda, so the compiler checks it: a misspelled field is a compile error, not a wrong answer. `std.table` (14.34) has the same methods for big data kept in columns.

```kavach
type Sale {
    region: String
    month: Int
    amount: Float
}

const sales = [
    Sale(region: "north", month: 1, amount: 120.0),
    Sale(region: "south", month: 1, amount: 80.0),
    Sale(region: "north", month: 2, amount: 100.0),
]

print(sales.count_by(.region))                                   // {"north": 2, "south": 1}
print(sales.group_by(.region).map_values(g => g.sum(.amount)))  // {"north": 220.0, "south": 80.0}
print(sales.mean(.amount) ?? 0.0)                                // 100.0
print(sales.max_by(.amount)?.month ?? 0)                         // 1
print(sales.map(s => (region: s.region, amount: s.amount)))      // two columns picked out
```

Lists:

| Method | Gives |
|---|---|
| `items.group_by(key) -> {K: [T]}` | The items for each `key(item)`: keys in the order first seen, items in their order. `sales.group_by(.region)` |
| `items.count_by(key) -> {K: Int}` | How many items have each key, in the order first seen: `words.count_by(w => w)` |
| `items.unique() -> [T]` | Each item once, in the order first seen: `[3, 1, 3].unique()` is `[3, 1]` |
| `items.unique_by(key) -> [T]` | The first item with each key, in order: `sales.unique_by(.region)` is one sale for each region |
| `items.min_by(f) -> T?`, `items.max_by(f) -> T?` | The item whose `f(item)` is smallest, or largest; the first one on a tie, and `none` for an empty list: `sales.min_by(.amount)` |
| `items.mean(f) -> Float?` | The average of `f(item)`, or `none` for an empty list: `sales.mean(.amount)` |
| `items.median(f) -> Float?` | The middle `f(item)` in sorted order; halfway between the two middle ones for an even count; `none` for an empty list |
| `items.var(f, correction: 1) -> Float?`, `items.std(f, correction: 1) -> Float?` | The variance of `f(item)`, divided by the count minus `correction`, and its square root, the standard deviation. The default, 1, gives the sample form, as pandas and R do; `correction: 0` gives the population form, as NumPy does. `none` when the count is not above `correction` |
| `items.describe(f) -> Summary?` | A summary of `f(item)`: `count`, `mean`, `std`, `min`, `q1`, `median`, `q3`, `max`; `none` for an empty list |
| `items.inner_join(other, key, other_key, row) -> [S]` | `row(a, b)` for each item `a` and item `b` of `other` with `key(a) == other_key(b)` |
| `items.left_join(other, key, other_key, row) -> [S]` | The same, and `row(a, none)` for each item `a` that matches nothing; `row` takes a `U?` |
| `items.pivot(row_key, column_key, cell) -> {R: {C: V}}` | For each `row_key(item)`, and in it each `column_key(item)`, `cell` of the list of items with both keys |

Dicts:

| Method | Gives |
|---|---|
| `d.map_values(f) -> {K: U}` | The same keys, in their order, each value through `f`: `{"a": [1, 2]}.map_values(g => g.len())` is `{"a": 2}` |
| `d.to_list() -> [(K, V)]` | The keys and values as pairs, in insertion order, so the list's methods apply: `counts.to_list().sort_by(p => -p.1)` |

**Keys.** A key is any value, compared as a dict compares its keys (2.2): `0.0` and `-0.0` are one key, and NaN is one key. A tuple groups by several keys at once: `sales.group_by(s => (s.region, s.month))`.

**Numbers.** `mean`, `median`, `var`, `std` and `describe` take a lambda that gives any number type (`Number`, 2.9), as `sum` does. Sums and means are exact, so they give the same bits in any order: on one core, on all cores and on the GPU (15.5):

- `mean` and `median` give a `Float` for whole numbers and `Float`, a `Float32` for `Float32`, and a `Decimal` for `Decimal`. `var`, `std` and `describe` give `Float`s for every number type.
- *Not built yet:* `sum(f)` of floats (`Float` or `Float32`) is the exact sum of the values, rounded once to the lambda's type ([decision 0135](../../decisions/0135-tables-and-grouping.md)). So `[1e20, 1.0, -1e20].sum(x => x)` is `1.0`, where adding in order gives `0.0`. A NaN, or infinities of both signs, give NaN; an infinity gives that infinity; an exact sum past the type's largest value gives an infinity. A sum of zeros is `-0.0` only when every value is `-0.0`, as IEEE 754 says.
- `mean` adds exactly too, with no overflow, and divides the exact sum by the count, rounding once: the result is the number nearest the true mean, for whole numbers, `Decimal`s and floats alike.
- `var` works out the mean first. Each squared difference from it is a float, and they are added exactly, as `sum` adds.
- `median` and `describe` sort as `sort` does: NaN after every number. A NaN makes `mean`, `var` and `std` NaN, as IEEE 754 does. Nothing is skipped without saying: to leave values out, `filter` them first.
- The quartiles `q1` and `q3`, and the median, are at 0.25, 0.5 and 0.75 of the way through the sorted values: the value at position `p × (count − 1)`, or the point that far between its two neighbors, as pandas, NumPy and R do by default.

`describe` gives an anonymous record (2.14), called `Summary` on this page. `std` is `none` for a single value:

```kavach
type Summary = (count: Int, mean: Float, std: Float?, min: Float, q1: Float, median: Float, q3: Float, max: Float)

const s = [2.0, 4.0, 4.0, 5.0].describe(x => x)
print(s?.median ?? 0.0)          // 4.0
print(s?.q1 ?? 0.0)              // 3.5: position 0.75, three quarters of the way from 2.0 to 4.0
```

**Joins.** `inner_join` and `left_join` give the joined rows in the order of `items`; an item's matches come in the order of `other`. Two items of `other` with one key give two rows, as SQL does. A join builds a dict of `other` by its key, so its time grows with the two lengths and the result, not their product. The keys are of one type. A right join is a left join written the other way round: `users.left_join(orders, .id, .user_id, row)`.

```kavach
type User {
    id: Int
    name: String
}

type Order {
    user_id: Int
    total: Float
}

const named = orders.inner_join(users, .user_id, .id, (o, u) => (name: u.name, total: o.total))
const listed = orders.left_join(users, .user_id, .id, (o, u) => (name: u?.name ?? "unknown", total: o.total))
```

**Pivots.** `pivot` means `group_by(row_key).map_values(g => g.group_by(column_key).map_values(cell))`. Each row's dict holds only the cells that have items, so a missing cell is read with `??`:

```kavach
const by_month = sales.pivot(.region, .month, g => g.sum(.amount))
print((by_month["north"] ?? {:})[2] ?? 0.0)     // 100.0
print((by_month["south"] ?? {:})[2] ?? 0.0)     // 0.0: no sale there
```

**One name for each job.** Picking columns out is `map` to an anonymous record (2.14), as the first example does; there is no `select`. A sum per key is `group_by(key).map_values(g => g.sum(f))`. Names other tools use are compile errors that name Kavach's: `sum_by` names that form and `sum(f)`, `value_counts` names `count_by`, `distinct` and `drop_duplicates` name `unique` and `unique_by`, `merge` and a `join` given lambdas name `inner_join`, `select` names `map`, and `average` and `avg` name `mean`. A range has them too, as it has every method that reads a list (2.2).

These replace `collections.unique(items)` and `collections.counts(items)` (14.9), now `items.unique()` and `items.count_by(x => x)`, and `math.mean(xs)` (14.5), now `xs.mean(x => x)`.
