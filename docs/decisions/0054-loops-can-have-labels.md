# 0054. Loops can have labels

- **Status:** Accepted
- **Date:** 2026-11-03
- **Replaces:** the "Labeled `break`" row of [decision 0013](0013-core-language-decisions.md)

## Context

[Decision 0013](0013-core-language-decisions.md) left labeled `break` out: to leave several nested loops at once, a program moved them into a function and returned. It noted that labels could be added later without breaking code.

Mithlesh asked for them (2026-11-03), the way JavaScript writes them, instead of `return`. Leaving an outer loop from an inner one is common (a search through a grid, a retry loop around a read), and a function made only to `return` from moves the loop away from the code that uses its result.

## Decision

**A `for` or `while` loop may have a label, and `break` and `continue` may name it,** as in JavaScript, Java, Go and Swift:

```kavach
outer: for row in grid {
    for cell in row {
        if cell == target {
            break outer          // leaves both loops
        }
    }
}
```

- The label is a name and a colon before `for` or `while`. `break outer` leaves the loop named `outer`; `continue outer` goes on with its next round.
- A bare `break` or `continue` still means the innermost loop.
- A label names a loop around the `break` in the same function. A lambda's body is a function of its own, so a `break` inside one can't leave a loop outside it.
- Naming a label that isn't around the `break` is a compile error that lists the labels that are; two loops around each other can't share a label.

## Consequences

- Nested loops can stop at once where they are written, with no flag and no helper function.
- The checker, the interpreter and both native backends learn labeled loops; the GPU backend's loops too.
- Breaking from a loop by name doesn't change the speed: it is the same jump as a plain `break`.

## Alternatives considered

- **Keep no labels (decision 0013):** simpler grammar, but Mithlesh wants the familiar form.
- **Rust's `'outer:`** or **Kotlin's `outer@`:** a sign more to learn, for nothing a plain name doesn't give.
- **A loop as a value (`const outer = for …`):** a loop isn't a value in Kavach, so it would read as something it isn't.
