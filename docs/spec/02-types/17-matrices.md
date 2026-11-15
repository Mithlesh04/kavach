# 2.17 Matrices

A `Matrix<T>` is a grid of numbers, `rows` × `cols`, kept row by row in one flat list. `a @ b` is the matrix product; `+`, `-` and `*` work item by item ([decision 0111](../../decisions/0111-matrices.md)).

```kavach
use std.matrix

const a = matrix.of([[1.0, 2.0], [3.0, 4.0]])
const b = matrix.of([[5.0, 6.0], [7.0, 8.0]])
print((a @ b).to_rows())     // [[19.0, 22.0], [43.0, 50.0]]
print((a * b).to_rows())     // [[5.0, 12.0], [21.0, 32.0]]
print(a.transpose().at(0, 1)) // 3.0
```

| Written | Gives |
|---|---|
| `matrix.of(rows: [[T]]) -> Matrix<T>` | The matrix with these rows; they must all be as long |
| `matrix.filled(rows, cols, value: T) -> Matrix<T>` | Every item `value` |
| `matrix.identity(n, one: T) -> Matrix<T>` | `n` × `n`, `one` on the diagonal and zeros elsewhere |
| `m.rows`, `m.cols` | The size |
| `m.at(i, j) -> T` | The item in row `i`, column `j`, both counted from 0 |
| `m.index(i, j) -> Int` | Where that item is in `m.items`: `m.items[m.index(i, j)] = v` changes it, in place when nothing else holds `m`, which must be a `let` |
| `m.row(i) -> [T]`, `m.col(j) -> [T]` | One row or one column |
| `m.to_rows() -> [[T]]`, `m.items -> [T]` | The rows, or every item row by row |
| `a + b`, `a - b`, `a * b`, `-a` | Item by item; `a` and `b` must be the same size |
| `m.scale(x: T) -> Matrix<T>` | Every item times `x` |
| `a @ b` | The product: `a.cols` must equal `b.rows`, and the result is `a.rows` × `b.cols` |
| `a.product(b, fast: true)` | The same product, worked out as fast as the machine allows (below) |
| `m.transpose()` | Rows become columns |
| `m.sum() -> T`, `m.sum_rows() -> [T]`, `m.sum_cols() -> [T]` | The sum of every item, of each row, of each column |
| `m.slice(row_start, row_end, col_start, col_end)` | The rows and columns from each start up to, not including, each end, as a new matrix |

- `T` is a number type: `Float`, `Float32` or a whole-number type (2.1).
- A size that doesn't fit breaks a `require` (7): `a + b` of different sizes, `a @ b` when `a.cols != b.rows` or `a` has no columns, an index out of range. Rows of different lengths in `matrix.of` panic, saying both lengths.
- Whole numbers overflow as they do anywhere (5.6): `@` panics on a sum that doesn't fit.

**The same bits everywhere.** By default each item of `a @ b` adds its products in one order, `k` from 0 up, each a multiply then an add, never fused. So a product gives the same bits on every machine and in every build, as a loop written out by hand would, while the work is still spread over the cores and the CPU's vector units, row by row. `a.product(b, fast: true)` may add in another order and fuse multiplies with adds, for the full speed of the machine: a `Float` result may then differ in its last bits from run to run on different machines. For whole numbers both give the same result.

`@` comes with the operator interfaces (2.16): `MatMul` is `a @ b`, `fn matmul(self, other: R) -> Out`, so a program's own types can have it too.

## Open questions

- Products on the GPU for large matrices (15.2), choosing the device by size as GPU `map` does, the data staying on the GPU between products.
- A matrix of a type that isn't a number is not yet a compile error; its first arithmetic or product panics. A compile error needs bounds on generics (2.9).
- Views that share their rows with the matrix they came from, instead of copies (`slice`, `transpose`), and the layout choice of to-do item 14.
- More than two dimensions (tensors).
