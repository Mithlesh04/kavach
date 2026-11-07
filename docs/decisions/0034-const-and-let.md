# 0034. `const` for values that never change, `let` for variables

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

Kavach declared a name with `let` (it never changes) or `var` (it can change). Mithlesh asked for the words most programmers already read this way: `const` for a constant, `let` for a variable, and no `var` (2026-11-02). JavaScript and TypeScript, the largest group of programmers today, use exactly these two words with exactly these meanings, so code reads as expected on first sight.

## Decision

1. `const name = value` declares a name that never changes. Assigning to it, or calling a method that changes it (`push`), is a compile error that says to use `let`.
2. `let name = value` declares a variable: it can be assigned, and its fields and items can be set.
3. `var` is no longer a keyword's meaning: writing it is a compile error that says to write `let`.
4. Unchanged: `if let` and `while let` bind the value inside an optional (that binding never changes); function parameters never change, like `const`; `const _ = f()` throws a result away on purpose.
5. Style: prefer `const`; use `let` only for a name that really changes.

## Consequences

- Every Kavach file, sample and spec example changes in the same commit: `let` becomes `const`, then `var` becomes `let`. The meaning of each program stays the same.
- `const` is not "computed at compile time" (as in C++ or Rust): it is an immutable binding, as in JavaScript. A value known while compiling may still be folded by the optimizer.
- Decision records written before this one keep their old examples, as a record of their time.

## Alternatives considered

- **Keep `let` and `var`** (Swift's words): readers from JavaScript, TypeScript and Kotlin read `let` as changeable; Mithlesh chose the JavaScript reading.
- **`const` and `var`**: `var` carries JavaScript's old function-scope surprises in many readers' minds.
