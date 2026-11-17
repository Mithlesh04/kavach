# stdlib/tensor/

`std.tensor` (spec [14.33](../../docs/spec/14-standard-library/33-tensor.md), [decision 0125](../../docs/decisions/0125-kavach-for-ai-work.md)): numbers in any number of dimensions.

| File | Contents |
|---|---|
| `tensor.kav` | `Tensor<T>`, its shapes checked as each operation runs; `+ - *` item by item and `@` through `std.matrix`; `full`, `zeros`, `ones`, `of`, `of_rows` |
