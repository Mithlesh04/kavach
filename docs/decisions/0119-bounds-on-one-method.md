# 0119. Bounds on one method: `where T: Floating`

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

`std.vec`'s vectors were `Vec2<T: Floating>`, so a grid could not use `Vec2<Int>`. Most of a vector's methods (`+`, `-`, `dot`, `cross`) work on any number, but a few (`length`, `normalized`) need a square root, so a whole-number vector can't have them. Kavach's bounds were on a type or a function as a whole. A maintainer chose, 2026-11-16, bounds on single methods over separate `IVec` types or waiting (to-do language request 9 (5)).

## Decision

- A method may add a bound to its type's parameter: `fn length(self) -> T where T: Floating`. The method is there only for the types that meet it.
- The bound is `Number` or `Floating` (spec 2.9). Interfaces as `where` bounds wait until a need shows.
- `where` is a word only on a signature's line, so existing names `where` keep working.
- The checker gives the parameter the bound while it checks the method's body, and checks each call once literals' types are known. A literal with no type of its own becomes a `Float`, as for a function's bound.
- `std.vec`'s `Vec2`, `Vec3` and `Vec4` become `T: Number`; `length`, `normalized`, `distance` and `lerp` take `where T: Floating`.

## Consequences

- One family of vector types serves grids and geometry: `Vec2<Int>` has `+`, `dot` and `length_squared`, and `Vec2<Float>` also has `length`.
- Code generation is unchanged: a generic method is compiled only for the types it is called with, and the checker refuses the others.

## Alternatives considered

- **Separate `IVec2`, `IVec3`, `IVec4`:** no language change, but two families of near-identical types to learn and keep in step. Rejected.
- **Interfaces as `where` bounds now:** more general, but nothing needs them yet. Left for later.
- **Waiting:** grids would keep using tuples. Rejected by a maintainer.
