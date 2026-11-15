# 0111. Matrices: `a @ b`, the same bits by default, `gemm` for speed

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

A matrix in Kavach is a `[[Float]]` today: a list of lists, each row its own allocation. Neither the CPU nor the GPU can multiply that at full speed, and every program writes its own product loop. Matrices serve the math speed goal (to-do item 1), simulation (Sakshi, item 10), games (item 9) and data and AI work. A maintainer asked for them on 2026-11-15 (item 12).

Three choices shape the rest, and a maintainer made them (2026-11-15):

1. **The operator.** `a @ b` is the matrix product and `*` multiplies item by item, as NumPy does (PEP 465). Writing `*` for the product reads well in a maths book, but in code it hides which product is meant.
2. **The same bits by default.** A product's sums can be added in many orders; a fast library picks one that suits the machine, so the last bits of a `Float` result change from machine to machine. Kavach's default keeps one order, so a product is repeatable everywhere; `fast: true` asks for full speed instead.
3. **The fast kernels from `gemm`** (pure Rust, MIT, by the author of `faer`), with the exact-order kernel our own.

## Decision

- **`Matrix<T>` is written in Kavach**, in `std.matrix` (spec 2.17): the size and one flat list of items, row by row. Its operators come from the interfaces of 2.16 (`Add`, `Sub`, `Mul`, `Neg`), and a new one, **`MatMul`**, for `@`, with the precedence of `*` (1.8). So `@` is not special to matrices, and a program's own types can have it.
- **The work is the runtime's**: the product, item-by-item arithmetic, sums and transposes run in Rust on the flat lists, by the items' type, so a matrix of `Float` is a plain buffer of `f64` there.
- **The exact product** adds each item's products in order, `k` from 0 up, a multiply then an add, never fused (Rust never fuses them unless asked). Rows go to different cores, and each row's loop runs over the columns, which the compiler turns into vector instructions without changing any item's order. Whole numbers check for overflow.
- **`fast: true`** uses `gemm` for `Float` and `Float32`: blocked, packed, fused and vectorized for the CPU it finds. It runs without `rayon`: each thread takes a band of rows. Whole numbers use the exact kernel either way.
- **The kernels live in `runtime/matrix`**, used by the interpreter and native code alike. `gemm` takes raw pointers, so the crate has one `unsafe` call, with a `SAFETY:` comment on the sizes it is given; nothing else there is `unsafe`.
- **Later stages**: products on the GPU for large matrices, views without copies, and tensors (spec 2.17's open questions).

## Consequences

- One new outside dependency, `gemm` (with its small helper crates), built without its `rayon` feature, and one more place with an `unsafe` call (AGENTS.md section 8).
- A 512 × 512 `Float` product took 7 ms exact and 4 ms with `fast` on the development machine (2026-11-15), both on every core.
- Two products to keep correct: the exact one is the reference that tests compare `fast: true` against, within a tolerance.
- `Matrix<String>` isn't a compile error until generics can ask for number types (2.9); its first arithmetic or product panics.
- `@` is now an operator, so the lexer, parser and checker learn it, as they learned the other operators of 2.16.

## Alternatives considered

- **A built-in type in the compiler**, like `Bytes`. Every method would need the checker, the interpreter and both backends to learn it. Written in Kavach, only the heavy loops are runtime functions.
- **`faer` whole**, for decompositions too. Larger than we need for products; `gemm` is the part of it that multiplies.
- **OpenBLAS or MKL.** Fast, but C or Fortran libraries to ship and link on every system, and neither gives the same bits on every machine.
