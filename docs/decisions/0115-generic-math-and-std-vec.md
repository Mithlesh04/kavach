# 0115. Generic math through `Number` and `Floating`, and vectors in `std.vec`

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

Games, simulations and graphics need vectors, matrices and quaternions: `Vec3`, `Mat4`, `Quat`. They need them fast on the CPU and inside `gpu fn`s, where GPUs have their own vector types and instructions. This is language request 9 (5) in the to-do.

A maintainer chose that the program picks the number type, `Vec3<Float32>` or `Vec3<Float>`, and that each use runs as fast as a type written for that number: "it's ok if compile time take little more time but runtime should be more faster" (2026-11-15).

Kavach couldn't do that: generic code can't add two `T`s. Spec 2.16 left "a generic function that asks for an operator" open. `Matrix<T>` (2.17) gets around it by calling the runtime for every operation, which is fine for large matrices and far too slow for a `Vec3` added a million times a frame.

## Decision

- **Two built-in bounds** for generic number code:
  - `T: Number`: every number type. Generic code may use `+ - * / %`, unary `-`, comparisons, number literals, and `T(x)` to convert any number to `T`.
  - `T: Floating`: `Float` and `Float32`. Also `math.sqrt`, `floor`, `sin` and the rest of `std.math` on `T`.
  - The bound is `Floating`, not `Float`, because `Float` is already the name of a type.
- **Each generic function is compiled once for each type it is used with** (monomorphization, already how generics compile, spec 2.9). A `Vec3<Float32>` add is three `f32` adds, as if written by hand.
- **The number types meet `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg` and `Comparable`**, so a bound written with operator interfaces accepts numbers and a program's own types alike. This settles spec 2.16's first open question.
- **`std.vec`** holds `Vec2<T>`, `Vec3<T>`, `Vec4<T>`, `Mat4<T>` and `Quat<T>`, written in Kavach over these bounds, with their operators through the operator interfaces.
- **On the GPU**, the compiler knows `std.vec`'s types of `Float32` and writes them as WGSL's own `vec2<f32>` to `mat4x4<f32>`. So `a + b`, `a.dot(b)` and `m @ v` in a `gpu fn` use the GPU's own instructions, and a vertex fn's `position` can be a `Vec4`. Other number types stay on the CPU.

## Consequences

- Generic number code is written once and is as fast as hand-written code for each type. Compile times grow with the number of types used, as a maintainer accepted.
- `std.vec` is plain Kavach, so it can be read, and a package can add its own types the same way.
- The WGSL backend gains vector and matrix types: a stage of its own, after the types work on the CPU.

## Alternatives considered

- **Only operator interfaces** (`T: Add + Mul + …`): works for a program's own types too, but long signatures, and nothing gives `sqrt` on `T`.
- **Two fixed copies** (`Vec3` of `Float32`, `Vec3d` of `Float`): no language change, but the program can't choose.
- **Runtime calls for each operation**, as `Matrix<T>` does: no language change, but too slow for small vectors.
- **Records whose methods are `gpu fn`s**, instead of WGSL's types: less compiler work, but slower shaders and no `Vec4` position.
