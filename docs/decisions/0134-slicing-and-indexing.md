# 0134. Slicing with `xs[a:b:step]`, `#k` from the end, and indexing through interfaces

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

Kavach could read one item (`xs[i]`) and cut out a part with a method: `.slice(start, end)` on lists, text and `Bytes`, `m.slice(r0, r1, c0, c1)` on matrices. There was no step, no way to count from the end, no comma form for many dimensions, and no way for a program's own type to be indexed. Operators already work for a program's own types through interfaces ([decision 0109](0109-operators-through-interfaces.md)); indexing did not.

A maintainer asked for Python's slices and for indexing of a program's own types (to-do 26, 2026-11-19), and made the choices below the same day.

### What other languages do

- **Python:** `a[start:stop:step]`, each part optional. A negative number counts from the end: `a[-1]` is the last item. A slice out of range is clamped; an index out of range raises an error. `slice.indices(len)` turns a slice into three plain numbers. A type gets `[]` through `__getitem__`, which receives a `slice` object, or a tuple for `a[1, 2]`.
- **Go:** `a[lo:hi]`, and `a[lo:hi:max]`, whose third part is the capacity of the new slice, not a step. No negative numbers, no step; a slice out of range panics. A slice shares its items with the array it came from.
- **Rust:** `&v[1..3]` with ranges (`..`, `a..`, `..b`). A type gets `[]` through the `Index` and `IndexMut` traits. Out of range panics; `v.get(1..3)` gives an `Option`. No negative numbers; a step is an iterator (`step_by`, `rev`).
- **Swift:** ranges `a..<b`, `a...b`, `a...`, and a type's own `subscript`. Out of range stops the program.
- **C#:** `^1` is the last item, `^0` the length; `a[1..^1]` is all but the first and last. A type with `Length` or `Count` and an `int` indexer gets `^k` for free: the compiler writes `a[a.Length - k]`.
- **JavaScript:** `a.slice(1, -1)` and `a.at(-1)` count from the end with negatives, but `a[-1]` is `undefined`: one meaning in a method, another in brackets.
- **NumPy:** commas for dimensions: `m[1, 2]`, `m[0:2, 1:]`, `m[:, -1]`. A number drops its dimension; a slice keeps it.
- **Lua and D:** Lua's `#t` is a table's length, so `t[#t]` is the last item. D writes the length as `$`: `a[$ - 1]`.

## Decision

1. **Slices are written as in Python:** `xs[start:end:step]`, each part optional, on lists, text, `Bytes`, text views, matrices, tensors and a program's own types (spec [2.19](../spec/02-types/19-indexing-and-slicing.md)). A slice is a new value. A slice out of range is clamped.
2. **The step is as in Python.** A negative step walks backwards: `xs[::-1]` is `xs` reversed. A step of 0 is a compile error when written, and a panic when worked out.
3. **From the end is written `#k`:** the position `len - k`. `xs[#1]` is the last item, `xs[#3:]` the last three, `xs[:#1]` all but the last, `m[#1, :]` the last row. `xs[#n:]` with `n` of 0 is empty.
4. **A plain number always counts from the start.** A negative index is out of range: a panic that names it, and `xs.get(i)` gives `none`. In a slice, a computed negative number is before the start, so it is clamped to 0, as any slice out of range is. A written negative number (`xs[-1]`) is a compile error that names `xs[#1]`.
5. **`.slice(...)` is removed, everywhere, for good:** lists, text, `Bytes`, matrices and views. So are the other methods that did a slice's job: `take(n)` (`xs[:n]`), `skip(n)` (`xs[n:]`), `reverse()` (`xs[::-1]`), text's `truncate(n)` (`s[:n]`) and `view(a, b)` (`v[a:b]`; a slice of a view is a view). Writing any of them is a compile error that names the slice form. One way to do one job. The matrix methods `at`, `row`, `col` and `index` go with `Matrix` itself, when `Tensor` becomes the one array type (to-do 28).
6. **Many dimensions take commas, as NumPy writes them:** `m[1, 2]`, `m[0:2, 1:]`, `m[:, #1]`, for matrices, tensors and a program's own types. Nested lists chain: `rows[1][2]`.
7. **A program's own types use interfaces, as operators do** (spec [2.16](../spec/02-types/16-operators.md)): `Index<Key, Out>` (`fn index(self, key: Key) -> Out`) for `x[k]`, `IndexSet` for `x[k] = v`, and `Slice` (`fn slice(self, part: Part) -> Self`). A `Part` holds `start`, `end` and `step` (`Int?`) and whether each end was written with `#`. `part.bounds(len)` turns it into three plain numbers, as Python's `slice.indices` does. Many dimensions give a tuple key.
8. **Text is indexed by characters**, as `len()` counts them: `s[1]` is a `Char`, `s[1:3]` is text, and `#k` works too: `s[#1]` is the last character.
9. **A slice can be set, with as many items:** `xs[1:3] = [7, 8]` replaces those items of a `let` list. Another length is a panic that names both lengths, so setting a slice never changes a list's length; `push`, `pop`, `+=` and `remove` do that.
10. **On the GPU too** (spec [15.4](../spec/15-gpu.md#154-slicing-on-the-gpu)): a slice of a list in GPU memory is made there, with no copy back, and `xs[a:b:step].map(f)` reads the slice where it is. Slices of matrices and tensors in GPU memory run there. A program's own `Index` and `Slice` methods may run in a `gpu fn` when they fit one. The results are those of the CPU, and an index out of range panics at the same line.

### Why `#k`, not negative numbers

A negative index changes its meaning with arithmetic. In Python, `xs[i - 1]` at `i = 0` quietly reads the last item instead of failing, and `xs[-n:]` at `n = 0` is the whole list instead of nothing. Both are common bugs, and neither shows in a test that misses the edge. In Kavach a computed number always counts from the start, so `xs[i - 1]` at `i = 0` panics, naming `-1`. Counting from the end is something you write on purpose, with `#`, and `xs[#n:]` at `n = 0` is empty, as "the last 0 items" should be.

### Why `#`, not another mark

- `^` (C#'s choice) is xor in Kavach (1.8): `xs[^1]` would read as an operator.
- `~` is bit-not, and Python's `~0` is the last item, so `~k` is off by one from `#k`. Readers who know Python would be misled.
- `$` (D's length) reads as "first" to anyone who knows shells, where `$1` is the first argument.
- `#` has no other meaning in Kavach: comments are `//` (1.2). It already means "length" in Lua, and "number" in plain writing, so `#1` reads as "number 1 from the end". `#k` means exactly what C#'s `^k` means.

### Why remove `.slice(...)` and the methods like it

Two ways to cut a list (`xs.slice(1, 3)` and `xs[1:3]`, or `xs.take(2)` and `xs[:2]`) mean two forms to read, teach and check. AI tools and people coming from Python, NumPy and Go write the bracket form first. Kavach has no users yet, so the method goes now, with no shim. The compile error names the new form, so old code is fixed in one step.

### Why setting a slice keeps the length

Python lets `xs[1:3] = ys` give `ys` any length, so the list grows or shrinks, and every later index moves. That hides bugs, and it is the one place a slice changes a list's length. In Kavach a list's length changes only through `push`, `pop`, `+=` and `remove`, which say so. With the lengths equal, a set slice is many `xs[i] = v` in one line, done in place.

### Why interfaces, and why `Self` from `Slice`

Operators already work this way (0109), so a reader learns one pattern. The interface is declared on the type, where `kavach outline` and `kavach query type` show it. `Slice` gives `Self`, so a slice of a `Row` is a `Row`, as a slice of a list is a list. Python passes a slice object and a tuple for many dimensions; `Part` and a tuple key are that design with types.

## Consequences

- One form for every cut: `xs[a:b:step]`. Code from Python and NumPy reads almost as written; only negative indexes change, to `#k`, and the compiler says so.
- The compiler work: the parser (`:` parts, `#`, commas in brackets), the checker (types of each form, the written `-1` and step `0` errors, the `.slice` error), the interpreter, native code and the GPU runtime.
- Every `.slice(` in `stdlib/`, `tests/`, `benchmarks/` and the website changes to the slice form, with the code.
- `Part` is a new built-in type, and `Index`, `IndexSet` and `Slice` are new built-in interfaces.
- `take`, `skip`, `reverse`, `truncate` and `view(a, b)` go, with `.slice`; their uses in `stdlib/`, `tests/` and the website change with the code.

## Alternatives considered

- **Python's negative indexes:** familiar, but they hide `xs[i - 1]` at `i = 0` and make `xs[-n:]` the whole list at `n = 0` (above).
- **Keep `.slice(start, end)`, `take`, `skip`, `reverse` and `truncate` beside the bracket form:** two ways for one job.
- **Setting a slice of any length, as Python allows:** see above.
- **No text indexing (`s.chars()[i]`):** makes a list to read one character.
- **Go's `a[lo:hi:max]`:** its third part is a capacity, which only makes sense when a slice shares its items. Kavach's values are copied on change (6.1), so a capacity has no meaning, and readers from Python would read it as a step.
- **Rust and Swift ranges (`xs[1..3]`):** Kavach already has `a..b` as a value (1.8), but a range has no step and no "from the end" without more syntax, and `xs[..]` reads worse than `xs[:]`.
- **C#'s `^k`:** the right meaning, the wrong character (above).
- **Only lists, no own types:** matrices, tensors and grids written in Kavach would need a method each, the problem 0109 solved for operators.
