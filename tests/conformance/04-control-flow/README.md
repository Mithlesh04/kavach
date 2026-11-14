# 4. Control flow

Conformance tests for spec chapter [4. Control flow](../../../docs/spec/04-control-flow.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 4.1 | [conditions-must-be-bool.kav](conditions-must-be-bool.kav) | Conditions must be Bool; there is no truthiness. | compile error |
| 4.1 | [if-is-an-expression.kav](if-is-an-expression.kav) | `if` is an expression; `else if` chains are allowed. | runs |
| 4.1 | [condition-with-equals-is-an-error.kav](condition-with-equals-is-an-error.kav) | A condition written with `=` is a compile error that says to write `==`. | compile error |
| 4.2 | [if-let.kav](if-let.kav) | `if let` runs a block only when an optional has a value. | runs |
| 4.3 | [match-must-cover-every-case.kav](match-must-cover-every-case.kav) | A match must cover every possible case. | compile error |
| 4.3 | [match-tuples.kav](match-tuples.kav) | Match patterns can take tuples apart. | runs |
| 4.3 | [match-values-and-ranges.kav](match-values-and-ranges.kav) | Match patterns include exact values, ranges, guards and `_`. | runs |
| 4.3 | [match-variants.kav](match-variants.kav) | Match patterns bind variant fields, rename them, ignore them, and take guards; first match wins. | runs |
| 4.3 | [a-match-arm-may-leave.kav](a-match-arm-may-leave.kav) | A `match` arm may leave instead of giving a value: `return`, `break` or `continue`; the `match` has the value of the arm taken. | runs |
| 2.6, 4.3 | [match-cases-are-written-with-their-type.kav](match-cases-are-written-with-their-type.kav) | A `match` case names its variant with its union's name; a bare case is a compile error. | compile error |
| 4.4 | [for-over-lists.kav](for-over-lists.kav) | `for` walks a list in order; enumerate() adds an index starting at 0. | runs |
| 4.4 | [for-over-a-map-a-call-gives.kav](for-over-a-map-a-call-gives.kav) | `for (key, value)` walks a map in insertion order, also when a call gives the map. | runs |
| 4.4 | [for-over-ranges.kav](for-over-ranges.kav) | `..` excludes the end; `..=` includes it. | runs |
| 4.4 | [for-underscore-discards.kav](for-underscore-discards.kav) | `for _ in …` discards the value and repeats the body. | runs |
| 4.4 | [big-loops-give-plain-results.kav](big-loops-give-plain-results.kav) | A loop's results don't depend on how it is run: a loop on all cores stores exactly what the plain loop stores (decision 0031). | runs |
| 4.4 | [big-loops-keep-their-panic-round.kav](big-loops-keep-their-panic-round.kav) | A loop panics at its first failing round, however it is run: a loop on all cores runs again serially to panic there. | panic |
| 4.4 | [big-sums-give-plain-results.kav](big-sums-give-plain-results.kav) | A sum's result doesn't depend on how the loop is run. | runs |
| 4.4 | [big-sums-keep-their-panic-round.kav](big-sums-keep-their-panic-round.kav) | A sum panics at its first failing round, however it is run: each chunk's prefix bounds catch a spike its total alone would hide. | panic |
| 4.4 | [big-modular-sums-give-plain-results.kav](big-modular-sums-give-plain-results.kav) | A modular sum's result doesn't depend on how the loop is run: `acc = (acc + v) % m` with a never-negative `v` is the flat sum `% m`. | runs |
| 4.4 | [modular-sums-anywhere-give-plain-results.kav](modular-sums-anywhere-give-plain-results.kav) | A sum taken by a remainder each round gives the plain loop's result after another such loop, inside an outer loop, by a parameter, and in tasks at once. | runs |
| 4.4 | [loop-one-past-the-end-warns.kav](loop-one-past-the-end-warns.kav) | A `for` over `0..=xs.len()` that reads `xs[i]` gets a warning; one that changes `xs` or leaves first gets none. | warning |
| 4.4 | [nested-loops-run-in-order.kav](nested-loops-run-in-order.kav) | Nested loops run every round in order, whatever the optimizer does: sums keep their order and effects happen one `j` at a time. | runs |
| 4.4, 5.6 | [nested-loops-panic-in-order.kav](nested-loops-panic-in-order.kav) | When a row is too short, the nested loops still panic at the same round, after the output of the rounds before it. | panic |
| 4.5 | [while-loop.kav](while-loop.kav) | `while` repeats while its condition is true. | runs |
| 4.6 | [break-and-continue.kav](break-and-continue.kav) | `continue` skips to the next iteration; `break` leaves the loop. | runs |
| 4.6 | [break-leaves-innermost-loop.kav](break-leaves-innermost-loop.kav) | `break` leaves only the innermost loop. | runs |
| 4.6 | [labeled-loops.kav](labeled-loops.kav) | A loop may have a label; `break name` leaves that loop and `continue name` goes on with its next round, from inside inner loops; a plain `break` still leaves the innermost. | runs |
| 4.6 | [labels-name-loops-around.kav](labels-name-loops-around.kav) | `break name` must name a loop around it, in the same function: a name no loop around it has is a compile error that lists the ones it has. | compile error |
| 4.7 | [match-arm-that-never-matches-warns.kav](match-arm-that-never-matches-warns.kav) | A `match` arm after one that matches everything, or after one with the same pattern, never matches: a warning. | warning |
| 4.7 | [else-if-that-is-never-true-warns.kav](else-if-that-is-never-true-warns.kav) | An `else if` whose condition can only be true when an earlier one is never runs: a warning. | warning |
| 4.7 | [while-that-never-ends-warns.kav](while-that-never-ends-warns.kav) | A `while` whose condition reads only variables the loop never changes, with nothing inside that leaves it, never ends: a warning; `while true` gets none. | warning |
| 4.8 | [a-block-given-to-a-call-is-a-plain-block.kav](a-block-given-to-a-call-is-a-plain-block.kav) | A block given to a call is a plain block: its value is the expression's; `try`, `return` and `break` act as without the call. | runs |
| 4.8 | [only-some-calls-take-a-block.kav](only-some-calls-take-a-block.kav) | Only the standard library functions that say so take a block, and they must have one. | compile error |
