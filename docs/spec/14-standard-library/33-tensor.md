# 14.33 std.tensor

A `Tensor<T>` holds numbers in any number of dimensions, for machine learning and science ([decision 0125](../../decisions/0125-kavach-for-ai-work.md)). Every operation checks its shapes as it runs, and names both shapes when they don't fit; every result is the same on every machine and at every thread count (2.1). It is written in Kavach, on lists and `Matrix<T>` (2.17), which does its products.

```kavach
use std.tensor

fn main() {
    const x = tensor.of_rows([[1.0, 2.0], [3.0, 4.0]])   // shape [2, 2]
    const w = tensor.ones<Float>([2, 3])
    const y = x @ w
    print(y.shape)                 // [2, 3]
    print(y.items)                 // [3.0, 3.0, 3.0, 7.0, 7.0, 7.0]
    print((x + x).sum())           // 20.0
    print(x.transpose().items)     // [1.0, 3.0, 2.0, 4.0]
}
```

| Call | Gives |
|---|---|
| `tensor.of(items)`, `tensor.of_rows(rows)` | A tensor of one dimension, or of two from its rows, every row as long |
| `tensor.full(shape, value)`, `tensor.zeros<T>(shape)`, `tensor.ones<T>(shape)` | A tensor of `shape`, every number `value`, 0 or 1 |
| `t.shape`, `t.items` | Its sizes, and its numbers, the last dimension's next to each other |
| `t.len()`, `t.rank()` | How many numbers it holds, and how many dimensions |
| `t.get(at)` | The number at `at`, one index per dimension |
| `t.reshape(shape)` | The same numbers in another shape with as many |
| `t.transpose()` | The last two dimensions swapped |
| `a + b`, `a - b`, `a * b` | Item by item, for two tensors of one shape |
| `a @ b` | The matrix product of the last two dimensions, for each of the others, which must match |
| `t.scale(k)`, `t.map(f)` | Each number times `k`, or through `f` |
| `t.sum()`, `t.max()` | The sum of every number, added in order; the largest, or `none` for no numbers |

- `T` is any number type (`T: Number`, 2.9): `Float32`, `Float`, the whole-number types, `Decimal`.
- A shape that doesn't fit is a panic naming both shapes: `` `@` needs [2, 2]'s last size (2) to match [3, 3]'s next to last (3)… ``.
- A tensor is a value, as a list is (6.1): a copy shares its numbers until one of them changes.

## Open questions

- Shapes in the type (`Tensor<Float32, [B, 768]>`), once generics take values (to-do 14).
- Broadcasting (`t + 1.0`, a `[3]` added to each row of a `[2, 3]`), reductions along one dimension (`sum(axis: 0)`), and `mean`.
- A `fast` form that lets products and sums use any order and fused multiply-adds, faster but not the same everywhere (decision 0125, D3).
- `BFloat16` and `Float16` for storage (D2), autodiff as a tape (D4), and running on the GPU (D5).
