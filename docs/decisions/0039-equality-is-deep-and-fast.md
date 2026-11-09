# 0039. Equality compares contents, ignores map order, and skips shared parts

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

Mithlesh asked whether `b == a` on large, deeply nested maps is slow, and what Kavach should do (2026-11-03).

Most languages compare contents deeply: Python, Java, Rust, Swift, Clojure. JavaScript compares objects by identity only, a common source of bugs. A deep comparison of two big values that really are equal must look at everything, in every language. What a language can make fast are the other cases: values that are copies of each other, and values that differ.

Kavach copies share memory until one of them changes (spec 6.1), so after `let b = a` the two are one value in memory. Before this decision the runtime still walked every nested item to compare them. It also copied each level's items before comparing them, compared sets in n² steps, and treated two maps with the same entries added in another order as different.

## Decision

**`==` compares contents, at every depth (spec 1.8). It does so as cheaply as it can:**

1. **Shared values are equal at once.** Two values that are the same in memory (a copy not yet changed, or a nested part two values still share) are equal without looking inside. A list holding `NaN` therefore equals itself when it is the same list, as in Python and Swift, though `NaN != NaN` as numbers.
2. **Maps compare like sets: the order of keys doesn't count.** `["a": 1, "b": 2] == ["b": 2, "a": 1]`, as in Python, Java and Rust. Iterating still follows insertion order (spec 2.2). Maps hash without order too, so equal maps hash the same as keys.
3. **Maps and sets look each key up by hash**, so comparing them takes time in proportion to their size, not its square.
4. **Nothing is copied while comparing**, and a list of plain numbers is compared as one block of memory.

## Consequences

- `b == a` after `let b = a` costs one step, however big `a` is. Comparing two values that differ usually stops early.
- Two separately built, equal values still cost a full walk, as in every language with deep equality.
- Code that relied on maps in another order being different changes meaning. None in the repository did.

## Alternatives considered

- **Identity only, as JavaScript does:** fast, but `[1] == [1]` would be false, and values are meant to behave like plain values.
- **A saved hash in each list, map and set** (Clojure): two different big values would usually differ in one step, at the cost of 8 bytes per value and work on every change. Kept for later, if a benchmark shows the need.
- **Keeping map order in `==`:** matches iteration order, but surprises readers coming from every other mainstream language.
