# runtime/matrix/

The work of `std.matrix` (spec 2.17, [decision 0111](../../docs/decisions/0111-matrices.md)): the matrix product, item-by-item arithmetic, sums and transposes on flat lists of numbers. The interpreter and native code both call it.

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-matrix`, over `gemm` |
| `src/lib.rs` | The number types, item-by-item arithmetic with overflow checks, sums, transposes |
| `src/product.rs` | The product: in one fixed order by default, rows spread over threads; through `gemm` with `fast` (its one `unsafe` call) |
| `src/tests.rs` | Products against a plain loop, bit for bit; `fast` within a tolerance; overflow; sizes |
