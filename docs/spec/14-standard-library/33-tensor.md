# 14.33 std.tensor

A `Tensor<T>` holds numbers in any number of dimensions. It is Kavach's one array type, as NumPy's `ndarray` is Python's: a vector is a tensor of one dimension, a matrix one of two ([decision 0136](../../decisions/0136-tensor-the-one-array-type.md)). It serves science, data work and machine learning ([decision 0125](../../decisions/0125-kavach-for-ai-work.md)).

Every operation checks its shapes, and names both shapes when they don't fit. Every result has the same bits on every machine, at every thread count, and on the CPU or the GPU (2.1).

```kavach
use std.tensor

fn main() {
    const x = tensor.of_rows([[1.0, 2.0], [3.0, 4.0]])   // shape [2, 2]
    const w = tensor.ones<Float>([2, 3])
    const y = x @ w
    print(y.shape)                 // [2, 3]
    print(y.items)                 // [3.0, 3.0, 3.0, 7.0, 7.0, 7.0]
    print((x + 1.0).sum())         // 14.0
    print(x.sum(axis: 0).items)    // [4.0, 6.0]: the sum of each column
    print(x.mean())                // 2.5
    print(x[1, #1])                // 4.0
}
```

- `T` is a number type (`Float`, `Float32`, a whole-number type or `Decimal`, 2.1) or `Bool`. A tensor of `Bool` is a **mask** (below). A tensor of any other type is a compile error.
- Arithmetic needs `T: Number`; `mean`, `std`, `var`, `percentile`, linear algebra and the FFT need `T: Floating` (2.9). A method used on another `T` is a compile error that names the bound.
- A tensor is a value, as a list is (6.1): a copy shares its numbers until one of them changes.
- The numbers are kept in one flat list, `t.items`, the last dimension's next to each other (row by row for a matrix).

## Making tensors

| Call | Gives |
|---|---|
| `tensor.of(items: [T]) -> Tensor<T, dims: 1>` | A tensor of one dimension |
| `tensor.of_rows(rows: [[T]]) -> Tensor<T, dims: 2>` | A matrix from its rows; rows of different lengths are a panic naming both lengths |
| `tensor.full(shape: [Int], value: T) -> Tensor<T>` | Every number `value` |
| `tensor.zeros<T>(shape: [Int])`, `tensor.ones<T>(shape: [Int])` | Every number 0, or 1 |
| `tensor.eye<T: Number>(n: Int) -> Tensor<T, dims: 2>` | `n` × `n`, 1 on the diagonal and 0 elsewhere |
| `tensor.arange<T: Number>(start: T, end: T, step: T = T(1)) -> Tensor<T, dims: 1>` | `start + i * step` for `i` from 0 up, while below `end` (above it for a negative `step`), as `start..end` counts |
| `tensor.linspace<T: Floating>(start: T, end: T, count: Int) -> Tensor<T, dims: 1>` | `count` numbers from `start` to `end`, both included, evenly spaced |
| `tensor.random<T: Floating>(shape: [Int]) -> Tensor<T> uses Random` | Each number from 0 up to, not including, 1, as `random.float()` (14.15) |
| `tensor.normal<T: Floating>(shape: [Int], mean: T = 0.0, std: T = 1.0) -> Tensor<T> uses Random` | Numbers from the normal distribution with this mean and standard deviation |
| `tensor.random_int(shape: [Int], from: Int, until: Int) -> Tensor<Int> uses Random` | Each a whole number from `from` up to, not including, `until`, as `random.int`; at most 65,536 numbers at a time, as `random.bytes(n)` (14.15) |

```kavach
print(tensor.arange(0, 10, 3).items)          // [0, 3, 6, 9]
print(tensor.arange(1.0, 0.0, -0.25).items)   // [1.0, 0.75, 0.5, 0.25]
print(tensor.linspace(0.0, 1.0, 5).items)     // [0.0, 0.25, 0.5, 0.75, 1.0]

random.seeded(7) {
    const noise = tensor.normal<Float32>([2, 3])   // the same six numbers on every machine
}
```

- A shape's sizes are 0 or more; a negative one is a panic naming the shape. `tensor.full([], 2.5)` has shape `[]`: one number and no dimensions.
- `arange`'s `step` of 0 is a panic. Each number is worked out as `start + i * step`, not by adding `step` again and again, so no rounding builds up.
- `linspace`'s number `i` is `start + T(i) * ((end - start) / T(count - 1))`, and the last is exactly `end`. A `count` of 1 gives `[start]`, of 0 an empty tensor; a negative one is a panic.
- Random tensors draw their numbers in order, through `std.random` (14.15). So `random.seeded(s) { … }` gives the same tensor on every machine and device, and under `kavach test` the `Random` fake gives repeatable ones (11.5). A `Float32` from `random` is never 1.0: a number that rounds up to 1.0 is drawn again. `normal` uses the Box–Muller method, with `std.math`'s functions, which give the same bits everywhere (decision 0122). It draws two numbers for each two it gives: `u = 1.0 - random.float()` and `v = random.float()` give `sqrt(-2 ln u) cos(2πv)`, then `sqrt(-2 ln u) sin(2πv)`, worked out as `Float`s; each `z` becomes `mean + std * T(z)`. For an odd count the last sine is not used.

## Shape, items and indexing

| Written | Gives |
|---|---|
| `t.shape -> [Int]`, `t.items -> [T]` | Its sizes, and its numbers, the last dimension's next to each other |
| `t.len() -> Int`, `t.dims() -> Int` | How many numbers it holds, and how many dimensions: `t.shape.len()` |
| `t.item() -> T` | The one number of a tensor that holds exactly one, of any shape; any other count is a panic naming the shape |
| `t[i, j, …] -> T` | The number at one index per dimension, each counted from 0; `#k` counts from the end (2.19). An index out of range is a panic naming it and the shape. It needs one number for each dimension: `t[1]` of a matrix is a panic that says to write `t[1, :]` |
| `t[0:2, :, #1]` | A slice, as a new tensor: a slice keeps its dimension, a number drops it, as in NumPy. Fewer parts than dimensions take the rest whole; more is a panic naming both |
| `t[i, j] = v` | Changes that number; `t` must be a `let` (2.19) |
| `t[0:2, :] = u` | Changes those numbers; `u` is broadcast to the slice's shape (below) (not built yet) |
| `t.get(at: [Int]) -> T?` | The number at `at`, or `none` when an index is out of range, as a list's `get` |
| `t.reshape(shape: [Int]) -> Tensor<T>` | The same numbers in another shape with as many; another count is a panic naming both shapes |
| `t.transpose() -> Tensor<T>` | The last two dimensions swapped; for a tensor of fewer than two, a compile error when its dimensions are known (below) and a panic otherwise |
| `t.map(f: fn(T) -> U) -> Tensor<U>` | Each number through `f`, the shape kept |
| `t.to_rows() -> [[T]]` | The rows of a matrix (a tensor of two dimensions) |
| `Tensor<U>(t)` | Each number converted to `U`, as `U(x)` converts one (2.1): `Tensor<Float>(counts)` |

```kavach
const t = tensor.arange(0, 6, 1).reshape([2, 3])   // [[0, 1, 2], [3, 4, 5]]
print(t[1, #1])               // 5
print(t[:, #1].items)         // [2, 5]: the last column
print(t[0:1, ::2].shape)      // [1, 2]
print(t.transpose().shape)    // [3, 2]
```

- A tensor is indexed with a key of any length, a list of `Part`s (2.16): each place between the commas is a number, `#k` or a slice, and `#k` counts from the end of its own dimension.
- A number given as an argument where a `Tensor<T>` is asked for is a tensor of shape `[]` holding it, as NumPy takes a number for an array: `tensor.where(t < 0.0, 0.0, t)`, `t.eq(4.0)`.

## The number of dimensions in the type

*Not built yet.*

The number of dimensions may be written in the type, as numbers in types are written: by name, after the item type (2.9, to-do 29).

```kavach
fn row_sums(m: Tensor<Float, dims: 2>) -> Tensor<Float, dims: 1> {
    m.sum(axis: 1)
}

const m = tensor.of_rows([[1.0, 2.0], [3.0, 4.0]])   // Tensor<Float, dims: 2>
print(row_sums(m).items)                             // [3.0, 7.0]
row_sums(tensor.of([1.0, 2.0]))   // error: `row_sums` takes a `Tensor<Float, dims: 2>`, and this has 1 dimension
```

- `Tensor<Float>` has any number of dimensions; `Tensor<Float, dims: 2>` has exactly two. `t.dims()` gives the number either way.
- A `Tensor<T, dims: n>` may be given wherever a `Tensor<T>` is asked for.
- A `Tensor<T>` becomes one with known dimensions through `try Tensor<T, dims: 2>(t)`, which fails when `t` has another number, as `try Int(text)` fails (5.2).
- **What the compiler checks.** Where every operand's number of dimensions is known, the result's is known too, and a wrong number is a compile error: giving a 2-D tensor where a 1-D one is asked for, `t.inv()` of a 1-D tensor, or `t.sum(axis: 2)` of a 2-D one. The rules for each result are in the table below.
- **What is checked as the program runs.** The sizes along each dimension, and the number of dimensions of a plain `Tensor<T>`. A size that doesn't fit is a panic naming both shapes: `` `@` needs [2, 2]'s last size (2) to match [3, 3]'s next to last (3)… ``.
- A shape written as a list literal has a known length, so `tensor.zeros<Float>([2, 3])` is a `Tensor<Float, dims: 2>`. A shape held in a variable gives a plain `Tensor<T>`, unless its type is `[Int]<len: n>`.
- With `dims: n` known, `t.shape` is an `[Int]<len: n>`.

| Operation | Dimensions of the result |
|---|---|
| `tensor.of`, `arange`, `linspace`, `t[mask]`, `t.unique()` | 1 |
| `tensor.of_rows`, `eye` | 2 |
| `full`, `zeros`, `ones`, `random`, `normal`, `random_int`, `reshape` | The length of the shape |
| `a + b` and the other item-by-item operators, comparisons, `tensor.where` | The most of the operands' (a number counts as 0) |
| `a @ b` | 2 and 2 give 2; 1 and 2, or 2 and 1, give 1; 1 and 1 give 0; more give the most of the two |
| A reduction with `axis:` (`sum`, `mean`, `argmax`, `norm`, `percentile` …) | One fewer; without `axis:` it gives a number, not a tensor |
| `transpose`, `map`, `clamp`, `cumsum`, `sort`, `argsort`, `concatenate`, `split`, `inv`, `fft` | The same |
| `tensor.stack` | One more than each part |
| `t[…]` | One fewer for each part that is a number |

## Item by item, and broadcasting

| Written | Gives |
|---|---|
| `a + b`, `a - b`, `a * b`, `a / b`, `a % b`, `a ** b` | Item by item, after broadcasting `a` and `b` to one shape |
| `-a` | Each number negated |
| `t + x`, `t * x`, … with a number `x: T` | The number used for every item, as a tensor of shape `[]` |
| `t.clip(lo, hi)` | A compile error that names `clamp` |
| `t.clamp(lo: T, hi: T) -> Tensor<T>` | Each number limited to `lo..=hi`, as a number's `clamp` (14.2) |

**Broadcasting** lets two tensors of different shapes work item by item, as in NumPy:

1. Line the two shapes up from the right.
2. The one with fewer dimensions gets sizes of 1 on its left.
3. Two sizes fit when they are equal, or when one of them is 1. The result takes the larger.
4. A size of 1 is used again for every place of the other size. Nothing is copied to do so.

Shapes that don't fit are a panic naming both: `` `+` can't broadcast [2, 3] with [2]: sizes 3 and 2 don't fit, and neither is 1 ``.

```kavach
const m = tensor.of_rows([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])   // [2, 3]
print((m + tensor.of([10.0, 20.0, 30.0])).to_rows())           // [[11.0, 22.0, 33.0], [14.0, 25.0, 36.0]]
const col = tensor.of([1.0, 2.0]).reshape([2, 1])              // [2, 1]
print((m * col).to_rows())                                     // [[1.0, 2.0, 3.0], [8.0, 10.0, 12.0]]
print((m / 2.0).items)                                         // [0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
print((m ** 2.0).sum())                                        // 91.0
```

- Both operands have one item type: a `Tensor<Int>` and a `Tensor<Float>` don't mix, as `Int` and `Float` don't (1.8). Convert one first: `Tensor<Float>(counts) / total`.
- `Tensor<T>` implements each operator twice (2.16): once for a tensor on the right (`Add<Tensor<T>>`) and once for a number (`Add<T>`). So `m + u` and `m + 1.0` are both `+`.
- The number goes on the right, as for any operator of a program's own type (2.16): `m * 2.0`. `2.0 * m` is a compile error that says to write `m * 2.0`.
- Each item works as its numbers would alone (1.8, 5.6). For whole numbers, `/` cuts off the fraction; division by zero, a negative power and overflow panic.
- *Not built yet:* That panic names the item's place: `overflow in \`*\` at [1, 2]`.
- `t += x` and the other `op=` forms are `t = t op x`, so `t` must be a `let`.

## Comparisons, masks and `where`

A comparison item by item gives a mask, a `Tensor<Bool>` of the broadcast shape. A mask picks numbers out, or chooses between two tensors. `< <= > >=` come through the `Compare` interface (2.16), for a tensor or a number on the right.

| Written | Gives |
|---|---|
| `a < b`, `a <= b`, `a > b`, `a >= b` | A mask: the comparison item by item; `b` is a tensor or a number (`t > 0.5`) |
| `a.eq(b)`, `a.ne(b)` | A mask: equal, or not equal, item by item |
| `m & n`, `m \| n`, `~m` | Masks combined item by item: both, either, not (not built yet) |
| `t[mask] -> Tensor<T, dims: 1>` | The numbers where `mask` is `true`, in order; `mask` has `t`'s shape |
| `t[mask] = x` | Sets those numbers to `x`; `t` must be a `let` |
| `tensor.where(mask, a, b) -> Tensor<T>` | `a`'s number where `mask` is `true`, else `b`'s; all three broadcast to one shape; `a` and `b` are tensors or numbers |
| `t.any(test)`, `t.all(test)`, `t.count(test)` | As for a list (14.2): whether any or every number passes `test`, or how many do |

```kavach
const t = tensor.of([3.0, -1.0, 4.0, -1.5])
print(t[t > 0.0].items)                           // [3.0, 4.0]
print(tensor.where(t < 0.0, 0.0, t).items)        // [3.0, 0.0, 4.0, 0.0]
print(t.eq(4.0).items)                            // [false, false, true, false]
print(t.count(x => x < 0.0))                      // 2
let u = t
u[u < 0.0] = 0.0                                  // u is [3.0, 0.0, 4.0, 0.0]
```

- `a == b` and `a != b` compare whole tensors, as for every type (1.8): the same shape and the same numbers, giving one `Bool`. Equality item by item is the method `eq` (and `ne`), so the two never mix.
- `Tensor<T>` has no order of its own (`Comparable`), so a list of tensors can't be sorted with `sort()`.
- A `NaN` is equal to nothing and neither below nor above any number, as IEEE 754 says: `t.eq(t)` is `false` where `t` is `NaN`.

## Reductions by axis

A reduction works on every number, or along one dimension, its **axis**, counted from 0. With `axis:`, the result has that dimension taken out: the sum of each column of a matrix is `m.sum(axis: 0)`, and of each row `m.sum(axis: 1)`.

| Written | Gives |
|---|---|
| `t.sum()`, `t.prod()` | The sum or the product of every number, in order: `T` |
| `t.min()`, `t.max()` | The smallest or largest number, or `none` for no numbers: `T?` |
| `t.argmin()`, `t.argmax()` | The place of the first smallest or largest in `t.items`, or `none` for no numbers: `Int?` |
| `t.mean()`, `t.var(correction: Int = 1)`, `t.std(correction: Int = 1)` | The mean, variance and standard deviation: `T?`, `none` when there are no more numbers than `correction` (for `mean`, no numbers). The variance divides by `len - correction`: by the count minus 1, as for lists; `correction: 0` is NumPy's form |
| `t.sum(axis: Int)`, and `prod`, `min`, `max`, `mean`, `var`, `std` | The same along one axis, as a tensor with that dimension taken out |
| `t.argmin(axis: Int)`, `t.argmax(axis: Int)` | The index along the axis of the first smallest or largest: a `Tensor<Int>` |
| `t.cumsum(axis: Int? = none)` | The running sums along the axis, the last one for `none`: the shape kept |

```kavach
const m = tensor.of_rows([[1.0, 5.0, 2.0], [4.0, 3.0, 6.0]])
print(m.sum(axis: 0).items)       // [5.0, 8.0, 8.0]: each column
print(m.max(axis: 1).items)       // [5.0, 6.0]: each row
print(m.argmax(axis: 1).items)    // [1, 2]
print(m.mean(axis: 0).items)      // [2.5, 4.0, 4.0]
print(m.cumsum().to_rows())       // [[1.0, 6.0, 8.0], [4.0, 7.0, 13.0]]
print(m.std() ?? 0.0)             // 1.8708286933869707
```

- **A float sum is exact, rounded once.** `sum` gives the true sum of its numbers, rounded once to `T`, as lists' sums do (to-do 27). No order changes it, so it has the same bits on every machine, core count and device. `mean` is that exact sum divided by the count, rounded once. `var` sums the squared distances from the mean the same way, then divides once. Each running sum of `cumsum` is exact, rounded once. A `NaN`, or `inf` with `-inf`, gives `NaN`; a result too large for `T` is `inf`, as IEEE 754 says.
- `prod` multiplies in order, from index 0 up.
- A reduction along an axis of size 0 is a panic naming the shape for `min`, `max`, `argmin`, `argmax` and `mean`, and so is `var` or `std` along an axis no longer than `correction`; `sum` gives 0 and `prod` 1 there.
- `NaN` comes after every number, as in a list's `min`, `max` and `sort` (14.2): `max` and `argmax` find a `NaN` if there is one; `min` and `argmin` find one only when every number is `NaN`.
- Whole numbers overflow as anywhere (5.6): `sum` and `prod` panic on a result that doesn't fit.
- *Not built yet:* That panic names the axis.
- An axis is from 0 up to `t.dims()`, not including it. Another one is a panic naming it. A written negative axis is a compile error, as `xs[-1]` is (2.19).
- *Not built yet:* An axis out of range is a compile error when the number of dimensions is known.
- `t.sum()` gives a `T` and `t.sum(axis: 0)` a tensor: `std.tensor` writes each reduction twice, once with `axis:`, and the call chooses by whether `axis:` is given. Only the standard library writes a method twice this way (2.16).

## Sorting, joining and splitting

| Written | Gives |
|---|---|
| `t.sort(axis: Int? = none)` | The numbers sorted along the axis, the last one for `none`, smallest first; `NaN` last (14.2) |
| `t.argsort(axis: Int? = none) -> Tensor<Int>` | The indexes that would sort it; equal numbers keep their order |
| `t.unique() -> Tensor<T, dims: 1>` | Every number once, sorted; all `NaN`s count as one, last |
| `t.percentile(q: Float)`, `t.percentile(q: Float, axis: Int)` | The `q`th percentile, `q` from 0 to 100, by linear interpolation between the two nearest numbers, as NumPy's default: `T?`, or a tensor along an axis |
| `tensor.concatenate(parts: [Tensor<T>], axis: Int = 0)` | The parts one after another along the axis; their other sizes must match |
| `tensor.stack(parts: [Tensor<T>], axis: Int = 0)` | The parts, all of one shape, along a new axis |
| `t.split(count: Int, axis: Int = 0) -> [Tensor<T>]` | `count` equal parts along the axis; a size that doesn't divide is a panic naming both |
| `t.split_at(points: [Int], axis: Int = 0) -> [Tensor<T>]` | Parts cut before each of `points`, which must go up |

```kavach
const t = tensor.of([3.0, 1.0, 2.0, 1.0])
print(t.sort().items)                 // [1.0, 1.0, 2.0, 3.0]
print(t.argsort().items)              // [1, 3, 2, 0]
print(t.unique().items)               // [1.0, 2.0, 3.0]
print(t.percentile(50.0))             // 1.5
const a = tensor.of([1, 2])
print(tensor.stack([a, a]).shape)               // [2, 2]
print(tensor.concatenate([a, a]).items)         // [1, 2, 1, 2]
print(tensor.arange(0, 6, 1).split(3)[1].items) // [2, 3]
```

- The sort is stable, so `argsort` gives the same indexes everywhere.
- `percentile` of no numbers is `none`, and along an axis of size 0 a panic. A `q` outside 0 to 100 is a panic naming it.
- `concatenate` and `stack` of an empty list are a panic: there is no shape to give.

## Linear algebra

The products and solvers of linear algebra take matrices: tensors of two dimensions, `Tensor<T, dims: 2>`, of `T: Floating` (`@` also takes others, below).

| Written | Gives |
|---|---|
| `a @ b` | The matrix product (2.16 `MatMul`), as NumPy's `matmul`: below |
| `a.matmul(b, fast: true)` | The same product, as fast as the machine allows (below) (not built yet) |
| `a.inv() -> Tensor<T, dims: 2> ! Singular` | The inverse of a square matrix; fails with `tensor.Singular` when it has none (not built yet) |
| `a.solve(b) -> Tensor<T> ! Singular` | The `x` with `a @ x == b`, for a square `a` and a `b` of 1 or 2 dimensions, without working out the inverse (not built yet) |
| `a.det() -> T` | The determinant of a square matrix (not built yet) |
| `t.norm()`, `t.norm(axis: Int)` | The square root of the sum of the squares (the Euclidean norm), of every number or along an axis (not built yet) |
| `a.eig() -> Eig<T>` | For a symmetric matrix: `values`, smallest first, and `vectors`, one column for each value (not built yet) |
| `a.svd() -> Svd<T>` | The singular value decomposition, `a == u @ diag(s) @ vt` up to rounding: `u`, `s` (largest first) and `vt` (not built yet) |

```kavach
const a = tensor.of_rows([[2.0, 1.0], [1.0, 3.0]])
const x = a.solve(tensor.of([3.0, 5.0])) catch tensor.zeros<Float>([2])
print(x.items)                    // [0.8, 1.4]
print(a.det())                    // 5.0
print(a.eig().values.shape)       // [2]: about 1.382 and 3.618, (5 ∓ √5) / 2
print(tensor.of([3.0, 4.0]).norm())   // 5.0
```

**`a @ b`** follows NumPy's `matmul`:

- Two matrices, `[n, k]` and `[k, m]`, give `[n, m]`. The inner sizes must match.
- *Not built yet:* A 1-D `a` is a row and a 1-D `b` a column; that dimension is taken out of the result. So a vector and a matrix give a vector, and two vectors of one length give their dot product, a tensor of shape `[]` (`(u @ v).item()`).
- With more than two dimensions, the last two are the matrices, and the others are a batch: `[3, 2, 2] @ [3, 2, 2]` is `[3, 2, 2]`, one product for each of the 3. *Not built yet:* batches of different shapes, broadcast as above: `[8, 2, 3] @ [3, 4]` is `[8, 2, 4]`.
- `@=` is `a = a @ b`.

**The same bits, or `fast`.** By default each number of `a @ b` adds its products in one order, `k` from 0 up, a multiply then an add, never fused (decision 0111). The work is still spread over the cores and the vector units, and a 2-D block of `Float` or `Float32` runs the exact kernel of `runtime/matrix`. `a.matmul(b, fast: true)` may add in another order and fuse multiplies with adds, through `gemm`, for the full speed of the machine; a float result may then differ in its last bits between machines. For whole numbers both give the same result, and overflow panics.

- `a @ b` needs `T: Number`; the other calls here need `T: Floating`.
- *Not built yet:* `Eig<T>` and `Svd<T>` are records of `std.tensor`, with the fields named above; `Singular` is its error type (5.1).
- *Not built yet:* `inv`, `solve` and `det` use LU decomposition with partial pivoting. `eig` uses the Jacobi method, `svd` the one-sided Jacobi method. Each is one fixed algorithm, so its result has the same bits everywhere.
- *Not built yet:* Each eigenvector and singular vector is turned so that its largest number (the first, if several) is positive. So the result is the same everywhere, not only up to a sign.
- *Not built yet:* `svd` gives the reduced form: for an `[m, n]` matrix and `k = min(m, n)`, `u` is `[m, k]`, `s` is `[k]` and `vt` is `[k, n]`.
- *Not built yet:* A matrix that isn't square where one is needed, or a non-symmetric one given to `eig`, is a panic naming its shape. `eig` checks symmetry exactly: `a[i, j] == a[j, i]` for every pair.
- *Not built yet:* `Singular` is reported when a pivot is exactly 0; `det` of such a matrix is 0.0.

## The FFT

*Not built yet.*

| Written | Gives |
|---|---|
| `t.fft(axis: Int = self.dims() - 1) -> Spectrum<T>` | The discrete Fourier transform of a real tensor along the axis |
| `s.fft(axis: …)`, `s.ifft(axis: …) -> Spectrum<T>` | The transform, or its inverse, of complex numbers |
| `tensor.Spectrum(re: Tensor<T>, im: Tensor<T>)` | Complex numbers as two tensors of one shape: the real parts and the imaginary parts |
| `s.abs() -> Tensor<T>` | Each number's size, `sqrt(re² + im²)` |

```kavach
const t = tensor.of([1.0, 0.0, -1.0, 0.0])
const s = t.fft()
print(s.re.items)                 // [0.0, 2.0, 0.0, 2.0]
print(s.im.items)                 // [0.0, 0.0, 0.0, 0.0]
print(s.ifft().re.items)          // [1.0, 0.0, -1.0, 0.0]
```

- As in NumPy, `fft` doesn't scale and `ifft` divides by the length.
- Any length works. A length that is a power of two uses the radix-2 algorithm; others use Bluestein's. The angles come from `std.math`'s `sin` and `cos`, so the result has the same bits everywhere (decision 0122).
- `T` is `Float` or `Float32`.

## On the GPU

*Not built yet.*

Tensors run on the GPU as lists do (chapter 15), with no code changed.

- An operation on a large tensor of `Float32`, `Int32`, `UInt32` or `Bool` may run on the GPU, chosen by size as a GPU `map` is (15.2). Its result stays in GPU memory, so the next operation starts there. Reading `t.items`, a number or `print(t)` brings it back.
- A tensor of `Float` (64 bits) stays on the CPU, as do `Decimal` and the other whole-number types: a `gpu fn` takes only those four types (15.1), and WGSL has no 64-bit floats.
- **The results are the CPU's, to the last bit** (decision 0125, D3). On the GPU a float sum is the same exact sum, rounded once, and `/`, and `std.math`'s functions in a `map` (`math.sqrt`, `math.exp` …), use Kavach's own exact routines, not the GPU's. An operation that a GPU can't work out with the CPU's bits runs on the CPU instead.
- Slices (`t[0:2, :]`) and masks of a tensor in GPU memory are made there.
- `KAVACH_GPU` and `[run] gpu` choose the device as for `map` (15.2). With `KAVACH_GPU=check`, tensor results are compared bit for bit.
- `a.matmul(b, fast: true)` may use the GPU's own fast products, with its own order of sums.

## NumPy's names

Each job has one name (14.2). Where NumPy's name differs, it is a compile error that names Kavach's:

| NumPy | Kavach |
|---|---|
| `t.ndim`, `t.rank()` | `t.dims()` |
| `t.size` | `t.len()` |
| `t.clip(lo, hi)` | `t.clamp(lo, hi)` |
| `t.astype(U)` | `Tensor<U>(t)` |
| `t.T` | `t.transpose()` |
| `a.dot(b)` | `a @ b` |
| `t.tolist()` | `t.items`, or `t.to_rows()` for a matrix |
| `tensor.concat`, `tensor.cat` | `tensor.concatenate` (not built yet) |
| `tensor.identity` | `tensor.eye` (not built yet) |
| `t.scale(k)` | `t * k` |
| `a.product(b, fast: true)` | `a.matmul(b, fast: true)` (not built yet) |

*Not built yet:* `use std.matrix` and the type `Matrix<T>`, which this module replaces (2.17), are compile errors that name `std.tensor` and `Tensor<T, dims: 2>`.

## Dimensions and shapes in the type

*Not built yet.*

From [decision 0137](../../decisions/0137-numbers-in-types.md), 2.9). `Tensor<Float>` has any number of dimensions; `Tensor<Float, dims: 2>` has exactly two; `Tensor<Float, shape: [3, 4]>` has exactly that shape, so two dimensions as well. What a type says is checked while compiling; what it leaves out is checked when the program runs, as above.

```kavach
const x: Tensor<Float, shape: [2, 2]> = tensor.of_rows([[1.0, 2.0], [3.0, 4.0]])
const w = tensor.ones<Float>([2, 3])     // a Tensor<Float, shape: [2, 3]>
const y = x @ w                          // a Tensor<Float, shape: [2, 3]>
const z = w @ x                          // compile error: `@` needs [2, 3]'s last size (3) to match [2, 2]'s next to last (2)

fn rows_of(m: Tensor<Float, dims: 2>) -> Int { m.shape[0] }
rows_of(tensor.of([1.0, 2.0]))           // compile error: a tensor of 1 dimension where 2 are needed

fn embed<b: Int>(x: Tensor<Float32, shape: [b, 768]>) -> Tensor<Float32, shape: [b, 64]> { … }
```

- A shape known while compiling is in the result's type: `tensor.zeros<Float>([3, 4])`, `of_rows` of a literal, `reshape([2, 6])`. A shape of 2 numbers worked out while running gives `dims: 2`.
- `a @ b` of two known shapes checks the inner sizes and gives `[m, n]`; of known `dims` only, it gives that many dimensions and checks the sizes when it runs. `transpose()` swaps the last two sizes in the type. `+`, `-` and `*` of two known shapes need the same shape.
- `dims` and `shape` together are a compile error: a shape says its dims.
- A tensor whose type says more may be given where one that says less is expected. The other way takes `try`: `try Tensor<Float, dims: 2>(t)` fails with a `SizeError` when `t` has another number of dimensions (2.9).
- The checker knows these rules for `std.tensor`'s own operations.

## Open questions

- **`&`, `|` and `~` on masks** need bit operators for a program's own types (2.16, Open questions). Until then `tensor.where` and `map` do their job.
- **`t[i]` of a tensor of more than one dimension.** NumPy gives row `i`. Here a key of numbers only gives a `T`, so it needs one number for each dimension, and `t[i, :]` gives the row. Once `dims:` is in the type (to-do 29), `t[i]` of a matrix could give the row.
- **`random_int`'s limit.** `tensor.random_int` makes at most 65,536 numbers at a time, as `random.bytes(n)` does, so the tests `kavach test` makes for it end within their budget (11.9). NumPy has no such limit; lifting it needs a way to bound generated shapes, or the work moved into the runtime.
- **Choices made where this page said nothing, closest to NumPy, for review:** `normal`'s order of draws (above); a `Float32` draw of 1.0 drawn again; `percentile` of numbers with a `NaN` is `NaN`, as NumPy's; `percentile` interpolates as NumPy's `_lerp`, from the upper number when the fraction is 0.5 or more; `split_at` panics on points outside `0` to the size; and `cumsum`, `sort` and `argsort` take `axis: Int? = none`, `none` meaning the last axis, as a default can't name `self`.
- **How `dims:` rules are written.** `@` and `sum(axis:)` work out the result's `dims` from the operands'. This needs arithmetic on numbers in types (`dims: n - 1`) or rules the checker knows for `std.tensor` (to-do 29).
- **Passing a plain `Tensor<T>` where `dims: 2` is asked for.** Written here as `try Tensor<T, dims: 2>(t)`, as to-do 29 does for limited lists; it could instead be checked as the program runs, with no `try`.
- **Exact routines on every GPU.** Some GPUs fuse a multiply with an add (15.2). Whether every vendor can be made not to, or such GPUs fall back to the CPU for floats, needs testing on real hardware.
- **Random tensors on the GPU.** Making them there with the CPU's numbers needs a generator that can jump ahead (a counter-based one, such as Philox). `random.seeded`'s generator (14.15) doesn't say which it is.
- **`cumsum` without an axis.** NumPy's flattens the tensor first; here it runs along the last axis, as `sort` does.
- **More of NumPy:** several axes at once (`axis: [0, 1]`), keeping the reduced dimension (`keepdims`), `cumprod`, other norms (`ord`), `eig` of matrices that aren't symmetric (complex results), `svd`'s full form, linear algebra on batches of matrices, and `unique`'s counts.
- **A `Complex` number type**, which would make `Spectrum<T>` a `Tensor<Complex<T>>`.
- A size left open in a shape's type (`shape: [_, 768]`), beyond a function's number parameter (2.9).
- A `fast` form for sums and the other reductions, not only for `@` (decision 0125, D3).
- `BFloat16` and `Float16` for storage (D2), and autodiff as a tape (D4).
- Views that share numbers with the tensor they came from (slices, `transpose`), and the layout choice of to-do 14.
