# stdlib/tensor/

`std.tensor` (spec [14.33](../../docs/spec/14-standard-library/33-tensor.md), decisions [0125](../../docs/decisions/0125-kavach-for-ai-work.md) and [0136](../../docs/decisions/0136-tensor-the-one-array-type.md)): numbers in any number of dimensions, Kavach's one array type.

| File | Contents |
|---|---|
| `tensor.kav` | `std.tensor`: `Tensor<T>`, its shape, items, indexing, slicing and masks; `of`, `of_rows`, `full`, `zeros`, `ones`, `eye`, `arange`, `linspace`, `random`, `normal`, `random_int`, `where`, `concatenate`, `stack` |
| `ops.kav` | `std.tensor.ops`: the operators item by item, with broadcasting, each for a tensor or a number on the right; comparisons giving masks; `eq`, `ne`, `clamp`; `@` through `std.matrix` |
| `reduce.kav` | `std.tensor.reduce`: `sum`, `prod`, `min`, `max`, `argmin`, `argmax`, `mean`, `var`, `std`, each of every number or along an axis; `cumsum` |
| `order.kav` | `std.tensor.order`: `sort`, `argsort`, `unique`, `percentile`, `split`, `split_at`, and the work of `concatenate` and `stack` |
| `shapes.kav` | `std.tensor.shapes`: counting, broadcasting, keys of any length, and the lanes along an axis |
| `exact.kav` | `std.tensor.exact`: float sums that are exact, rounded once (decision 0135) |
