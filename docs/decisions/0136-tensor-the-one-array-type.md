# 0136. `Tensor` is the one array type; `Matrix` goes

- **Status:** Accepted
- **Date:** 2026-11-19
- **Replaces:** the `Matrix<T>` type and `std.matrix` of [decision 0111](0111-matrices.md). Its `@` operator, the `MatMul` interface, the same-bits product and the `gemm` kernels for `fast: true` stay, now on tensors.

## Context

Kavach has two array types. `Matrix<T>` (spec 2.17, decision 0111) holds numbers in two dimensions. `Tensor<T>` (spec 14.33, [decision 0125](0125-kavach-for-ai-work.md)) holds them in any number, and does its products through `Matrix`. For two dimensions they do the same job under two names, with two sets of methods to learn and keep correct.

A maintainer asked for array work like NumPy (to-do 28, 2026-11-19). Today's tensor has `of`, `full`, `zeros`, `ones`, `reshape`, `transpose`, `+ - *` item by item, `@`, `scale`, `map`, `sum`, `max` and `get`. It lacks most of what NumPy users reach for: ranges and random arrays, broadcasting, `/` and `**`, masks, reductions by axis, sorting, joining and splitting, linear algebra and the FFT.

### What other languages do

- **NumPy** had both `ndarray` and `np.matrix`, a 2-D class where `*` meant the matrix product. Mixing the two was a steady source of bugs: the same code gave other results for a matrix and an array. Python 3.5 added `@` (PEP 465), partly so `np.matrix` was no longer needed. NumPy's documentation now says not to use `np.matrix`, "even for linear algebra", and it warns on use (`PendingDeprecationWarning` since NumPy 1.15, 2018). scikit-learn and SciPy moved off it.
- **PyTorch** (`torch.Tensor`) and **JAX** (`jax.Array`) have one array type and never had a matrix class. A matrix is a tensor with `dim() == 2`; `@`, `inv` and `svd` take any tensor with at least two dimensions, and treat the others as a batch.
- **Julia** has one `Array{T, N}`, with the number of dimensions `N` in the type. `Matrix{T}` is only another name for `Array{T, 2}`, and `Vector{T}` for `Array{T, 1}`. So a function can ask for a matrix and the compiler checks it, with no second type.
- **Rust's `ndarray`** does the same: `Array<A, D>`, where `D` is `Ix2` for exactly two dimensions or `IxDyn` for any number, checked when the program runs.
- **Eigen** (C++) has `Matrix<T, Rows, Cols>`, where each size is a number or `Dynamic`. Fixed sizes, such as 4 × 4, live on the stack with unrolled loops; dynamic ones on the heap. Eigen also has a separate `Array` class for item-by-item work, converted with `.array()` and `.matrix()`: two classes for one grid of numbers, which its users often find confusing.

The lesson from all five: one array type, with the matrix as its two-dimensional case; the number of dimensions may be in the type, as Julia and `ndarray` do, so the compiler can still check what a `Matrix` type checked.

## Decision

A maintainer chose (2026-11-19):

1. **`Tensor<T>` is Kavach's one array type**, like NumPy's `ndarray`. Every NumPy-style method lives on it, in `std.tensor` (spec 14.33).
2. **`Matrix<T>` and `std.matrix` go.** A two-dimensional tensor does their job, `@` included. Spec 2.17 becomes a pointer to 14.33. `use std.matrix` and the name `Matrix` are compile errors that name `std.tensor` and `Tensor<T, dims: 2>`.
3. **The number of dimensions may be in the type**, as to-do 29 writes numbers in types (types by position, numbers by name):

   ```kavach
   fn row_sums(m: Tensor<Float, dims: 2>) -> Tensor<Float, dims: 1> {
       m.sum(axis: 1)
   }
   ```

   `Tensor<Float>` has any number of dimensions; `Tensor<Float, dims: 2>` has exactly two. `dims` is chosen over `rank`, as NumPy says `ndim` and PyTorch `dim()`. Where the dimensions are known, the compiler checks them: `a @ b` of two 2-D tensors is 2-D, `t.sum(axis: 0)` has one fewer, and a 2-D tensor given where a 1-D one is asked for is a compile error. Where they aren't known, each operation checks them as it runs, as today (decision 0125, D1).
4. **The methods come in this order**, each step spec first, then code and tests:
   1. `arange`, `linspace`, random tensors;
   2. broadcasting, `/` and `**` item by item, comparisons and boolean masks, `where`, `clip` (written `clamp`, below);
   3. reductions by axis: `sum`, `mean`, `std`, `var`, `min`, `max`, `argmax`, `argmin`, `cumsum`, `prod`;
   4. `sort`, `argsort`, `concatenate`, `stack`, `split`, `unique`, `percentile`;
   5. linear algebra: `inv`, `solve`, `det`, `norm`, `eig`, `svd`;
   6. `fft`.
5. **The same results everywhere**, on the CPU and on the GPU. Every operation gives the same bits on every machine, at every thread count and on either device (decisions 0122 and 0125, D3). The GPU is used where it can give the CPU's bits; `fast: true` asks for speed instead.
6. **The fast product of decision 0111 stays.** `a @ b` of two `Float` or `Float32` tensors runs the exact kernels of `runtime/matrix` on each 2-D block, and `a.matmul(b, fast: true)` runs `gemm`, as `a.product(b, fast: true)` did.

Names follow Kavach's rule of one name per job (14.2): where Kavach already has a name, a tensor uses it, and NumPy's name is a compile error that names it. So NumPy's `clip` is `clamp`, as on numbers, and `t.clip(…)` is a compile error naming `clamp` (a maintainer confirmed, 2026-11-19); `ndim` and `rank` are `dims()`; `astype` is a conversion, `Tensor<Float>(t)`.

### What the drafts left open, decided

A maintainer answered the draft's open questions on 2026-11-19:

7. **An operator interface may be implemented more than once**, for different right operands, chosen by the right operand's type (spec 2.16): `Add<Tensor<T>>` and `Add<T>`, so `t + u` and `t + 1.0` are both `+`. This replaces decision [0109](0109-operators-through-interfaces.md)'s "one other per operator".
8. **`Pow`** is the interface for `**`, and **`Compare<R, Out>`** (`lt`, `le`, `gt`, `ge`) gives `< <= > >=` any result, so `t > 0.5` is a `Tensor<Bool>`. A type has `Comparable` or `Compare`, not both. `==` stays whole-value equality; equality item by item is `t.eq(u)` and `t.ne(u)`.
9. **Tensors of `Float` (64 bits) stay on the CPU.** `Float32`, `Int32`, `UInt32` and `Bool` tensors may go to the GPU, the types a `gpu fn` takes.
10. **`var` and `std` divide by the count minus 1** by default, with `correction:` (`correction: 0` is NumPy's form), for lists and tensors alike.
11. **A sum or mean of floats is the exact sum, rounded once**, as for lists (to-do 27). No order changes it, so it has the same bits on the CPU, on every core count and on the GPU.

### Moving from `Matrix` to `Tensor`

| `Matrix` (2.17, removed) | `Tensor` (14.33) |
|---|---|
| `use std.matrix` | `use std.tensor` |
| `Matrix<T>` | `Tensor<T, dims: 2>` |
| `matrix.of(rows)` | `tensor.of_rows(rows)` |
| `matrix.filled(rows, cols, value)` | `tensor.full([rows, cols], value)` |
| `matrix.identity(n, one)` | `tensor.eye<T>(n)` |
| `m.rows`, `m.cols` | `t.shape[0]`, `t.shape[1]` (`t.shape` is `[rows, cols]`) |
| `m.at(i, j)` | `t[i, j]` |
| `m.index(i, j)`, then `m.items[k] = v` | `t[i, j] = v` |
| `m.row(i)`, `m.col(j)` (lists) | `t[i, :]`, `t[:, j]` (1-D tensors; `.items` for the list) |
| `m.to_rows()`, `m.items` | `t.to_rows()`, `t.items` |
| `a + b`, `a - b`, `a * b`, `-a` | The same, and now `/` and `**`, with broadcasting |
| `m.scale(x)` | `t * x` |
| `a @ b`, `a @= b` | The same |
| `a.product(b, fast: true)` | `a.matmul(b, fast: true)`: `@` with the fast path |
| `m.transpose()` | `t.transpose()` (the last two dimensions swapped) |
| `m.sum()` | `t.sum()` |
| `m.sum_rows()`, `m.sum_cols()` | `t.sum(axis: 1)`, `t.sum(axis: 0)` |
| `m.slice(r0, r1, c0, c1)` | `t[r0:r1, c0:c1]` (spec 2.19, decision 0134) |

Today's tensor changes in three places too: `t.rank()` is `t.dims()`; `t.scale(k)` is `t * k`; `t.get(at)` gives `none` for an index out of range, as a list's `get` does, while `t[i, j]` panics.

## Consequences

- One type to learn, document and keep correct. Code written for NumPy, PyTorch or JAX reads almost the same in Kavach.
- `std.matrix`, its runtime calls and its tests move to `std.tensor`. The kernels in `runtime/matrix` stay; `std.tensor` calls them directly instead of through `std.matrix`.
- Linear algebra and the FFT, which [platform.md](../vision/platform.md) put in packages, move into `std.tensor`. Each is one fixed algorithm, so its result has the same bits everywhere.
- Sparse matrices stay a package. `std.vec`'s small `Mat3` and `Mat4` (spec 14.28) stay too: fixed-size values for graphics, which are WGSL's own types on the GPU, not arrays.
- `dims:` in the type needs numbers in types (to-do 29) first. Until then every tensor is `Tensor<T>`, and shapes are checked as the program runs.
- Same bits on the GPU cost speed: a float sum is worked out exactly there, and `/`, `sqrt` and `exp` use Kavach's own exact routines, not the GPU's. `fast: true` is the way out for products.
- An exact sum costs more than adding in order on one core, but can be split over every core and the GPU without changing a bit.
- The language grows in spec 2.16: an interface implemented more than once, `Pow` and `Compare`. Indexing with a mask and `&`, `|`, `~` on masks stay open (spec 14.33).
- Decisions 0115, 0122 and 0125 mention `Matrix<T>` as it was when they were written; they are not changed.

## Alternatives considered

- **Keep both, `Matrix` as the fast 2-D type.** Two names for one job, and two sets of methods; NumPy's experience with `np.matrix` shows the cost.
- **Keep `Matrix` as another name for `Tensor<T, dims: 2>`**, as Julia does. Kavach keeps one name per job, so the type's own name with `dims: 2` says the same with nothing new to learn.
- **`rank` for the number of dimensions.** Used by older TensorFlow, but in linear algebra a matrix's rank is something else; `dims` matches NumPy's `ndim` and PyTorch's `dim()`.
- **Sizes in the type now (`shape: [3, 4]`, as Eigen's fixed sizes).** Checks more, but most shapes are known only when the program runs; `dims:` is the part almost always known. `shape:` stays possible later (to-do 29).
- **Linear algebra and the FFT in packages**, as platform.md planned. A NumPy user expects them with the array, and only the standard library can promise the same bits on CPU and GPU.
