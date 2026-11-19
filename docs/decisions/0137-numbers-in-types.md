# 0137. Numbers in types: types positional, numbers named

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

Many types carry a number that matters: a hash is 32 bytes, a register is 12 bits, a port is from 1 to 65,535, money has 2 places, a vector has 3 items, a matrix is 3 × 4, a buffer holds at most 16. Until now Kavach checked these only while the program ran, or not at all: `List.capacity(n)` puts the limit on the value (spec 2.2), `std.tensor` checks shapes as it runs (decision 0125, D1), and a `[Float]` of 2 items given where 3 are meant is found by a test, if one exists. Bit widths other than 8, 16, 32 and 64 did not exist, which chip design needs (to-do item 10 (8)).

Several to-do items asked for numbers in types, each in its own form: `Simd<T, N>` (item 14), shapes in tensors (item 28, decision 0125), `UInt<N>` and `Bits<N>` (item 10). A maintainer chose, 2026-11-19 (to-do item 29), **one general mechanism** for built-in types and a program's own alike, with one rule chosen to avoid confusion.

### How other languages do it

- **Rust const generics** (stable since 1.51, 2021): `struct Grid<T, const W: usize, const H: usize>`, used as `Grid<f32, 4, 3>`. Whole numbers, `bool` and `char` only. Arithmetic in a type (`[T; N + 1]`) is still unstable. Arrays have their own form, `[T; N]`. `std::simd::Simd<f32, 8>` is positional too.
- **C++ templates:** non-type parameters, `std::array<float, 3>`, `std::bitset<12>`, positional. Errors come when a template is used, often long.
- **Zig comptime:** types are values made by functions run while compiling: `fn Matrix(comptime T: type, comptime rows: usize, comptime cols: usize) type`. Any bit width, `u12`, `i24`, up to 65,535 bits. Arrays `[3]f32`, vectors `@Vector(8, f32)`.
- **Ada:** ranges and decimals are types of their own: `type Port is range 1 .. 65_535;`, `type Money is delta 0.01 digits 10;`. A value out of range raises `Constraint_Error` when the program runs.
- **Haskell type-level naturals** (`GHC.TypeLits`, `vector-sized`'s `Vector n a`): full arithmetic on numbers in types, through `DataKinds` and `KnownNat`. Powerful, and the error messages are hard to read.
- **Eigen** (C++): `Matrix<float, 3, 4>` is checked while compiling; `Dynamic` sizes are checked when the program runs. `Matrix3f` names the common sizes.
- **Verilog and SystemVerilog:** `logic [11:0] addr;` gives any width. A wider value put into a narrower one is cut without an error (only lint tools warn).
- **SQL:** `NUMERIC(10, 2)`, precision then scale, positional. Which number is which is a common mistake.

What they share: positional numbers. `Simd<f32, 8>`, `Bytes<32>`, `Int<24>` and `List<f32, 16>` each have one number, and it means lanes, length, bits and a limit. The reader must know each type to read it.

## Decision

1. **Types are positional, numbers are named**, as named arguments are in calls (spec 3.2). In `<…>`, type arguments come first, in order; each number comes after them with its name, in any order. A number without its name, or a type with a name, is a compile error that shows the named form.
2. **The places.** The same rule serves every type that takes numbers:

   | Written | Means |
   |---|---|
   | `Int<bits: 24>`, `UInt<bits: 12>` | A whole number of exactly that many bits; `Int8` is `Int<bits: 8>`, `Int` is `Int<bits: 64>` |
   | `Int<range: 1..=65535>` | A whole number that is always in the range |
   | `Decimal<digits: 10, places: 2>` | At most 10 digits, 2 after the point |
   | `[Float]<len: 3>`, `List<Float, len: 3>` | A list of exactly 3 items |
   | `[Float]<max: 16>`, `List<Float, max: 16>` | A list of at most 16 items |
   | `{String: Int}<max: 100>`, `Dict<String, Int, max: 100>` | A dict of at most 100 keys |
   | `Set<Int, max: 10>`, `Ring<Float, max: 100>` | A set of at most 10; a ring of the last 100 |
   | `Bytes<len: 32>` | Exactly 32 bytes; `crypto.sha256` gives one |
   | `Tensor<Float, dims: 2>`, `Tensor<Float, shape: [3, 4]>` | Exactly two dimensions; exactly that shape |
   | `Simd<Float32, lanes: 8>` | 8 numbers worked on at once (to-do item 14) |
   | `type Grid<T, width: Int, height: Int>` | A program's own type, used as `Grid<Float, width: 4, height: 3>` |

3. **`<…>` belongs to the type just before it.** A collection's limit comes right after its type, in either spelling: `[Int]<max: 2>` or `List<Int, max: 2>`. So `{String: [Int]<max: 5>}` is a dict of lists of at most 5, and `{String: Int<bits: 8>}` a dict of 8-bit values. There is no `[Int, max: 2]` form: it reads as a list literal of two items.
4. **The fixed-length list is `[Float]<len: 3>`.** To-do item 29 first listed `[Float, len: 3]`. That form breaks rule 3 and looks like a list literal, so it is written `[Float]<len: 3>` or `List<Float, len: 3>`, as the limits are. `[Float, len: 3]` is a compile error that shows `[Float]<len: 3>`.
5. **Item types may be left out** and are inferred, as for `let a = []`: `let a: List<max: 2> = []`.
6. **A program's own types and functions** declare number parameters after their type parameters: `type Grid<T, width: Int, height: Int>`, `fn dot<n: Int>(a: [Float]<len: n>, b: [Float]<len: n>) -> Float`. Inside, a number parameter is a constant. A number argument is a literal, a constant worked out while compiling, or a number parameter in scope. A number parameter may have a default, as a function's parameter can.
7. **A limit known when writing the code goes in the type.** `List.capacity(n)` (and `Dict`, `Set`, `Ring`) stays only for a limit known while running. `List.capacity(5)` with a number known while compiling is a compile error naming `List<max: 5>`.
8. **Into a type with numbers only through `try`.** A plain value becomes a limited or fixed one only through `try List<Int, max: 2>(xs)` (or `catch`), which fails with a `SizeError` when it doesn't fit, as `try Int(text)` fails with a `ParseError` (spec 2.1) and `try Dump(...)` with an `InvariantError` (spec 7.3). The other way needs nothing: a `Bytes<len: 32>` may be given where `Bytes` is expected. Number conversions (`UInt<bits: 12>(x)`, `Int<range: 1..=65535>(x)`) panic when the value doesn't fit, as `Int8(300)` does today.

### What is checked when

The compiler checks every number it can see; the rest is checked when the program runs.

| Checked while compiling | Checked while running |
|---|---|
| A number argument's name and value; a missing one | A sum or product of `bits` or `range` values that doesn't fit: a panic, as overflow is (spec 5.6) |
| A literal that doesn't fit: `UInt<bits: 12>` = 4096, a list literal longer than `max`, `1.234` into `places: 2` | A push past `max`: a panic; `is_full()` checks first |
| Two types with different numbers given for each other | `try List<Int, max: 2>(xs)` and the other `try` conversions: a `SizeError` |
| An index written as a number past a `len`: `v[3]` of `[Float]<len: 3>` | An index worked out while running, as today |
| `push` on a `len` list; `List.capacity(5)` with a known number | A conversion `UInt<bits: 12>(x)` that doesn't fit: a panic |
| The inner sizes of `a @ b` when both shapes are in the types | Shapes and dims a type leaves out, as `std.tensor` does today |

## Consequences

- Mistakes in sizes, widths and limits are found while compiling, where the numbers are known, with no cost while running: a type is compiled once for each set of numbers, and the numbers are not stored in each value.
- One form to learn for every type, and a reader never has to know which number a type means: the name says it.
- Chip design and hardware registers get any bit width, up to 65,535 bits, so to-do item 10 (8)'s `Int128` question is answered by `Int<bits: 128>`.
- Spec examples and code that use `List.capacity(3)` with a constant must change to the type form. The tests, examples, standard library and website need the same change when this is built.
- `Matrix` (spec 2.17) is not changed here: to-do item 28 replaces it with `Tensor<T, dims: 2>`.
- New work: the checker must carry numbers in types, the parser must read `<…>` after `[T]` and `{K: V}`, and the native back end must store widths that are not 8, 16, 32 or 64 bits.

## Alternatives considered

- **Positional numbers** (`Grid<Float, 4, 3>`, `Int<24>`, `Decimal<10, 2>`), as Rust, C++, Eigen and SQL write them. Shorter, but the same number means different things in different types, and `Int<24>` can't say whether 24 is bits or a range. `Grid<Float, 4, 3>` hides which is the width. Rejected.
- **Rust's `[T; N]`** for fixed-length lists. A second form only for lists, a `;` in a language without semicolons, and no room for `max`, `bits` or a shape. Rejected.
- **`[Float, len: 3]` and `[Int, max: 2]`.** Read as list literals, and break "`<…>` belongs to the type before it". Rejected (rule 4).
- **Numbers in the name** (`Int24`, `Vec3`, `Mat4`, `F32x8`, `__m256`): a new name for every number, and generic code can't take the number as a parameter. The existing `Int8` … `UInt64` stay, as short spellings of `bits` types. Rejected for new types.
- **Zig's comptime functions that make types.** The most general, but types become code run while compiling, and errors point inside that code. Kavach keeps types declarative. Rejected.
- **Haskell's type-level arithmetic.** Arithmetic in number arguments (`len: n + 1`) is left open (spec 2.9 Open questions); Rust has not stabilized it either.
- **Ada-style declarations** (`type Port is range 1 .. 65535`). Clear, but a second mechanism beside generics. `Int<range: 1..=65535>` is the same idea through the one rule. Rejected.
- **Checking only while running**, as today's `List.capacity` and `std.tensor` do. Errors come late, in production. Kept only for numbers known while running.
