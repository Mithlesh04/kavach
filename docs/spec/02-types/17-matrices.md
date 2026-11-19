# 2.17 Matrices

A matrix is a tensor of two dimensions, `Tensor<T, dims: 2>`, in `std.tensor` ([14.33](../14-standard-library/33-tensor.md)). `Tensor` is Kavach's one array type; the `Matrix<T>` type and `std.matrix` that this section described are gone ([decision 0136](../../decisions/0136-tensor-the-one-array-type.md), replacing the type of [decision 0111](../../decisions/0111-matrices.md)).

```kavach
use std.tensor

const a = tensor.of_rows([[1.0, 2.0], [3.0, 4.0]])   // Tensor<Float, dims: 2>
const b = tensor.of_rows([[5.0, 6.0], [7.0, 8.0]])
print((a @ b).to_rows())      // [[19.0, 22.0], [43.0, 50.0]]
print((a * b).to_rows())      // [[5.0, 12.0], [21.0, 32.0]]
print(a.transpose()[0, 1])    // 3.0
```

- `a @ b`, its same-bits product and `a.matmul(b, fast: true)` are described in 14.33, under Linear algebra.
- `use std.matrix` and `Matrix<T>` are compile errors that name `std.tensor` and `Tensor<T, dims: 2>`.
- Decision 0136 has a table from every `Matrix` call to its tensor form: `matrix.of(rows)` is `tensor.of_rows(rows)`, `m.at(i, j)` is `m[i, j]`, `m.sum_rows()` is `m.sum(axis: 1)`, and so on.
