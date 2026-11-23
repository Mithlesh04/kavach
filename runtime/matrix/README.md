# runtime/matrix/

The matrix product of `std.tensor`'s `a @ b` (spec 14.33, decisions [0111](../../docs/decisions/0111-matrices.md) and [0136](../../docs/decisions/0136-tensor-the-one-array-type.md)) on flat lists of numbers, row by row. The interpreter and native code both call it; `std.tensor` reaches it as `matrix.product_id`, which only the standard library may call.

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-matrix`, over `gemm` |
| `src/lib.rs` | The number types, with overflow checks |
| `src/product.rs` | The product: in one fixed order by default, rows spread over threads; through `gemm` with `fast` (its one `unsafe` call) |
| `src/tests.rs` | Products against a plain loop, bit for bit; `fast` within a tolerance; overflow |
