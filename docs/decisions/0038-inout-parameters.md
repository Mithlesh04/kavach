# 0038. `inout` parameters: a function changes its caller's variable, safely

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

Mithlesh asked for pointers: taking an address, dereferencing it, changing through it (2026-11-02). Most of what programs use pointers for is letting a function change the caller's variable: a counter bumped, two values swapped, a list filled in. Kavach's values are copied, not shared (spec 6), so a function can't do that today except by returning the new value.

A pointer to an ordinary variable would give two names for one value. That is where use-after-free, data races and "who changed my list?" come from, which Kavach exists to rule out. Raw pointers stay where they are: `Ptr<T>` inside `unsafe`, for C (spec 6.4).

## Decision

**A parameter marked `inout` is copied in and copied back out: the function changes its own copy, and its last value goes back to the caller's place when it returns (spec 3.9).**

1. The argument is a place the caller could assign to: a `let` variable, or a field or list item of one. No marker at the call site: the function's signature says it.
2. The same variable can't appear twice in one call when one of them is `inout`, so a function never sees two names for one value.
3. When the function fails, nothing is written back.
4. A function with `inout` parameters isn't a function value.
5. In native code the function returns the final values too, and the caller stores them: no pointers to locals, and nothing changes in the backends.

## Consequences

- Swapping, bumping and filling in place read as they do in Swift or C#, without the aliasing bugs of C pointers.
- Copy in and copy out is cheap: values are reference counted, so passing in is a count, and a change copies only when the caller's value is still shared (spec 6.1).
- Pointer syntax on `Ptr<T>` inside `unsafe` (`*p`, `p + 1`) comes as its own step.

## Alternatives considered

- **References to variables (`&x`, `&mut x`):** the general tool, but they bring aliasing and lifetimes, which Kavach keeps out of safe code.
- **A marker at the call site (`bump(&count)`, `ref count`):** visible at the call, but it reads like a pointer, and the signature already says it; editors show it on hover.
- **Writing back even when the function fails** (as Swift does): the caller would see a half-done change after an error; keeping the old value is simpler to reason about.
