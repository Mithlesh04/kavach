# 0149. `is` checks a value's type, and narrows it

- **Status:** Accepted (details for a maintainer's review)
- **Date:** 2026-11-27

## Context

A maintainer asked (2026-11-27) for a way to check a value's type against a declared one, `a.type == Name`. `type` is a keyword, so that can't be the syntax. A check only matters where the type is known only when the program runs: an interface value (2.15), an inline union (2.6) and a declared union's variant. Until now an interface value had "no cast back to the concrete type" ([decision 0029](0029-interface-values.md)), and only `match` could tell an inline union's members apart.

Other languages: Kotlin `a is Name` (the value is then a `Name`), C# `a is Name n`, Swift `as?`, TypeScript `instanceof`, Python `isinstance`, Go `a.(Name)` and type switches, Java `instanceof Name n`. Most modern ones narrow: after the check, the value is used as that type.

## Decision

A maintainer chose (2026-11-27):

1. **`x is T`** gives a `Bool`: whether `x` holds a `T`. `T` is a type an interface value can hold, a member of an inline union, or a variant of a declared union (`shape is Shape.Circle`). `is` is a contextual keyword, a keyword only there (1.6).
2. **It narrows.** Inside `if x is T { … }`, and in the rest of a condition joined by `and`, `x` is a `T`, so a `T`'s fields and methods can be used. For an inline union, `else` narrows to the members left. Narrowing holds for a name the block doesn't assign to.
3. **`reflect.type_of(x) == T`** compares with a type name, which stands for its `Type` (14.32) there.
4. **A check the compiler can already answer** (`b is Name` where `b` is a `Name`) is a warning: always true, or always false.

This replaces decision 0029's point that an interface value has no cast back to its concrete type.

## Consequences

- Interface values can be told apart without a closed union, as other languages allow; a union and `match` stay the way to cover every case.
- The checker gains narrowing for a block, which inline unions need too (2.6).

## Alternatives considered

- **Only `type_of(a) == Name`:** no new keyword, but no narrowing, so using the value as that type still needs a `match`.
- **Only `match`:** what existed; complete and checked for coverage, but heavy for one test.
