# 2.19 Indexing and slicing

`xs[i]` reads one item. `xs[start:end:step]` is a **slice**: a new value with the items from `start` up to, not including, `end`, taking every `step`th one. `#k` counts from the end ([decision 0134](../../decisions/0134-slicing-and-indexing.md)).

```kavach
const xs = [10, 20, 30, 40, 50]
print(xs[0])            // 10
print(xs[#1])           // 50: the last item
print(xs[1:3])          // [20, 30]
print(xs[#2:])          // [40, 50]: the last two
print(xs[:#1])          // [10, 20, 30, 40]: all but the last
print(xs[::2])          // [10, 30, 50]
print(xs[::-1])         // [50, 40, 30, 20, 10]
print("kavach"[0:3])    // kav
```

| Value | `x[i]` | `x[start:end:step]` |
|---|---|---|
| A list `[T]` | `T` | `[T]` |
| `String` | `Char`, counted in characters, as `len` counts | `String` |
| `Bytes` (2.13) | `UInt8` | `Bytes` |
| `StrView` (2.18) | `Char` | `StrView`: a view of the same string |
| `Tensor<T>` (14.33) | one number per dimension | below |
| A dict `{K: V}` | `d[key]`: `V?` (2.2) | a compile error: a dict has keys, not positions |
| A program's own type | through `Index` (2.16) | through `Slice` (2.16) |

## Slices

- **Each part is optional.** A missing `start` is the first item, a missing `end` is past the last, and a missing `step` is 1: `xs[2:]`, `xs[:2]`, `xs[::3]`. `xs[:]` is all of `xs`. The parts are `Int`s.
- **A slice out of range is clamped**, never a panic. A part past the end counts as the end, and a computed part below 0 counts as before the first item. With a positive step, a `start` at or after `end` gives an empty slice. `part.bounds(len)` (2.16) gives the exact numbers.

  ```kavach
  const xs = [10, 20, 30]
  print(xs[1:99])       // [20, 30]
  print(xs[5:])         // []
  print(xs[2:1])        // []
  const a = -2
  print(xs[a:])         // [10, 20, 30]: -2 is before the start
  ```

- **The step works as in Python.** A negative step walks backwards: it starts at `start` and stops before it reaches `end`. With a negative step, a missing `start` is the last item, and a missing `end` is before the first. A step of 0 is a compile error when it is written, and a panic when it is worked out: `the step of a slice can't be 0`.

  ```kavach
  const xs = [10, 20, 30, 40, 50]
  print(xs[3:0:-1])     // [40, 30, 20]
  print(xs[::-2])       // [50, 30, 10]
  print(xs[::0])        // error: the step of a slice can't be 0
  ```

- **A slice is a new value.** Changing it never changes `xs`, and changing `xs` never changes it (6.1).
- **Text** is indexed and cut by characters, as `len()` counts them. `s[i]` is a `Char`, `s[#1]` the last one, and `s[a:b]` is text:

  ```kavach
  const s = "héllo"
  print(s[1])           // é
  print(s[#1])          // o
  print(s[1:3])         // él
  print(s[::-1])        // olléh
  print(s[9])           // panic: index 9 is out of range: the text has 5 characters
  ```

- The parts are worked out left to right, after the value being cut: `xs`, then `start`, `end` and `step`.

## From the end: `#k`

`#k` is the position `k` from the end: `len - k`, where `len` is the length of the value being indexed. It is written in an index or at either end of a slice.

```kavach
const xs = [10, 20, 30, 40, 50]
const n = 0
print(xs[#2])           // 40
print(xs[#3:#1])        // [30, 40]
print(xs[#n:])          // []: the last 0 items
print(xs[:#n])          // [10, 20, 30, 40, 50]
print(xs[#n + 1])       // 50: `#(n + 1)` is `#1`
```

- `#` covers the whole part after it, up to `:`, `,` or `]`: `xs[#n + 1]` is `xs[xs.len() - (n + 1)]`.
- `#0` as an index is out of range, as `xs[xs.len()]` is. As the end of a slice it is the end.
- `#` is written only inside `[ ]`; anywhere else it is a compile error.

## A plain number counts from the start

A number without `#` always counts from the start, so arithmetic never changes what an index means.

- **A negative index is out of range.** `xs[i]` with `i` of `-1` panics, naming it: `index -1 is out of range: the list has 5 items`. `xs.get(-1)` gives `none` (2.2).
- **A written negative number is a compile error** that names the `#` form: `xs[-1]` says to write `xs[#1]`, and `xs[-2:]` says to write `xs[#2:]`.
- So `xs[i - 1]` at `i = 0` panics, instead of reading the last item, and `xs[#n:]` at `n = 0` is empty, instead of the whole list.

```kavach
fn before(xs: [Int], i: Int) -> Int? { xs.get(i - 1) }
fn item_before(xs: [Int], i: Int) -> Int { xs[i - 1] }

print(before([10, 20, 30], 0))        // none
print(item_before([10, 20, 30], 0))   // panic: index -1 is out of range: the list has 3 items
const xs = [10, 20, 30]
print(xs[-1])                         // error: Kavach counts from the end with `#`: write `xs[#1]`
```

`#k` works where an item is set, too: `xs[#1] = v` changes the last item of a `let` list (2.2).

## Setting a slice

`xs[a:b:step] = ys` replaces the items of a `let` list that the slice names with the items of `ys`, in order. `ys` must have as many items as the slice: another length is a panic that names both lengths, so setting a slice never makes a list longer or shorter. `push`, `pop`, `+=` and `remove` do that.

```kavach
let xs = [1, 2, 3, 4, 5]
xs[1:3] = [7, 8]                // [1, 7, 8, 4, 5]
xs[#2:] = [0, 0]                // [1, 7, 8, 0, 0]
xs[::2] = [9, 9, 9]             // [9, 7, 9, 0, 9]
xs[0:2] = [6]                   // panic: the slice has 2 items, but 1 was given
```

- The slice is clamped first, as any slice is (above), and then the lengths are compared: `xs[3:99] = [1, 2]` sets the last two items of a list of 5.
- It is done in place when nothing else holds `xs`, as `xs[i] = v` is (6.1).

## Many dimensions

A value with many dimensions takes one part for each, separated by commas, as NumPy writes them. A number picks one place, and that dimension is gone from the result; a slice keeps the dimension.

*Not built yet:* tensors (14.33) take one part per dimension:

```kavach
use std.tensor

const t = tensor.of_rows([[1, 2, 3], [4, 5, 6], [7, 8, 9]])
print(t[1, 2])                  // 6
print(t[#1, :].items)           // [7, 8, 9]: the last row
print(t[:, #1].items)           // [3, 6, 9]: the last column
print(t[0:2, 1:].to_rows())     // [[2, 3], [5, 6]]
```

| Written on a `Tensor<T>` | Gives |
|---|---|
| `t[i, j, …]`, a number for every dimension | `T`: that number (not built yet) |
| any part a slice | `Tensor<T>`: a new tensor, without the dimensions a number picked (not built yet) |

- Each part follows the rules above: numbers out of range panic, slices are clamped, and `#k` counts from the end of its own dimension.
- A program's own type gets a tuple key (2.16).
- **Nested lists chain:** a list of lists is a list, one dimension at a time. `rows[1][2]` is item 2 of row 1, and `rows[1:3][0]` is row 1. `rows[1, 2]` on a list is a compile error that names `rows[1][2]`.

## On the GPU

*Not built yet.*

A slice of a list in GPU memory is made there, and `xs[a:b:step].map(f)` reads the slice where it is (15.4). The results are those of the CPU.

## No methods that slice

A slice is written only with brackets. Each method below did a slice's job, so it is gone, and writing it is a compile error that names the bracket form:

| Written | The error names |
|---|---|
| `xs.slice(1, 3)`, also on text, `Bytes` and views | `xs[1:3]` |
| `xs.take(n)` | `xs[:n]` |
| `xs.skip(n)` | `xs[n:]` |
| `xs.reverse()` | `xs[::-1]` |
| `s.truncate(n)` | `s[:n]` |
| `v.view(a, b)` on a view | `v[a:b]`, a view of the same string |
| `s.view(a, b)` on a `String` | `s.view()[a:b]` |

- A negative `n` is clamped, as in any slice: `xs[:n]` with `n` of -1 is `[]`, as `take(-1)` was.
- A program's own type may still have a method named `slice`: it is the method of `Slice` (2.16).
- `Matrix` goes, with its `slice`, `at`, `row` and `col` (2.17, decision 0136): a tensor's slices do their jobs.

## Open questions

- **`Json` lists:** `j[#1]` and `j[1:3]` for a `Json` list (2.12), whose reads give `Null` instead of panicking.
- **Slices that share:** a slice of a big list could share its items until either changes, as Go's and NumPy's slices do, instead of copying at once.
