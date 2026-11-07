# 0012. Return types are always written

- **Status:** Accepted
- **Date:** 2026-10-30
- **Amends:** [0002](0002-static-types-with-inference.md), which required return types only on `pub` functions

## Context

The spec contradicted itself. Section 3.1 said a function with no `->` returns nothing; section 2.10 said a private function's return type is inferred. For `fn double(x: Int) { x * 2 }`, the two rules give different answers. Writing conformance tests exposed the conflict.

## Decision

- A function with no `->` returns nothing, whether it is `pub` or private.
- A function that returns a value must write `-> Type`.
- If a function without `->` ends with a value, that is a compile error, and the error shows the fix (`-> Int`).
- Lambdas and local variables are still fully inferred.

(Spec 2.10 and 3.1.)

## Consequences

- One rule instead of two, and no guessing for readers: every signature says whether the function returns something, and what.
- Signatures stay the contract that people and AI tools review, for private code too.
- A forgotten `-> Int` can no longer silently drop a value. The compiler catches it and suggests the fix.
- Slightly more typing for small private helpers. Lambdas cover the cases where that would hurt most.

## Alternatives considered

- **Infer return types of private functions:** shorter, but then a missing `->` means "infer" in one place and "nothing" in another, and changing one line in the body can silently change the function's type.
- **Infer return types everywhere (as in many scripting languages):** the signature would no longer tell reviewers what a function returns.
