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
| 4.9 | [big-loops-give-plain-results.kav](big-loops-give-plain-results.kav) | An `on cores` loop's results don't depend on how it is run: it stores exactly what the plain loop stores (decision 0126). | runs |
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
| 4.8 | [a-block-after-another-call-is-refused.kav](a-block-after-another-call-is-refused.kav) | A block after a call that doesn't take one is a compile error that names the rule. | compile error |
| 1.4, 4.5, 4.6 | [a-let-in-a-loop-body-starts-anew-each-round.kav](a-let-in-a-loop-body-starts-anew-each-round.kav) | A `let` declared in a loop's body is made again each round from its first value, whether the last round ended normally, with `continue` or with `break`; outside variables keep their changes. | runs |
| 1.4, 4.5 | [a-name-made-in-a-loop-ends-with-the-loop.kav](a-name-made-in-a-loop-ends-with-the-loop.kav) | A name declared inside a loop's body belongs to that body: using it after the loop is an unknown name. | compile error |
| 1.4, 4.2, 4.3, 4.4 | [unused-loop-and-pattern-names-warn.kav](unused-loop-and-pattern-names-warn.kav) | A `for` variable, an `if let` name or a name a `match` pattern takes that is never used gets a warning; one starting with `_` gets none. | warning |
| 4.2 | [an-if-let-name-cannot-change.kav](an-if-let-name-cannot-change.kav) | The name `if let` binds never changes (decision 0034): assigning to it is a compile error. | compile error |
| 4.2, 1.4 | [an-if-let-name-hides-only-inside-its-block.kav](an-if-let-name-hides-only-inside-its-block.kav) | An `if let` name may reuse an outside name: it hides that name only inside its block, and the outside one is unchanged after it. | runs |
| 4.3, 1.4 | [match-bindings-hide-only-inside-their-arm.kav](match-bindings-hide-only-inside-their-arm.kav) | A match arm's binding may reuse an outside name: it hides it in that arm's guard and value only; a guard may read outside names too. | runs |
| 4.8 | [a-block-given-to-a-call-changes-lists-and-fields-around-it.kav](a-block-given-to-a-call-changes-lists-and-fields-around-it.kav) | A block given to a call is a plain block, not a lambda: pushing to an outside list, setting a field or a map key changes the variable around it. | runs |
| 4.8, 1.4 | [a-name-made-in-a-block-given-to-a-call-ends-with-it.kav](a-name-made-in-a-block-given-to-a-call-ends-with-it.kav) | A block given to a call is a block: a name declared in it can't be used after it. | compile error |
| 4.3 | [match-covers-nested-variants.kav](match-covers-nested-variants.kav) | Arms that differ in one field cover a variant when, between them, they cover that field's type: no catch-all arm is needed. | runs |
| 4.3 | [match-on-an-optional-may-list-its-variants.kav](match-on-an-optional-may-list-its-variants.kav) | A `match` on an optional may list `none` and the variants of the value inside, which match a present value. | runs |
| 4.9 | [on-cores-gives-the-one-core-result.kav](on-cores-gives-the-one-core-result.kav) | An `on cores` loop gives the result of the same loop on one core: each round writes its own slot, and whole numbers added with `+=` add up the same in any order. | runs |
| 4.9 | [on-cores-refuses-print.kav](on-cores-refuses-print.kav) | An `on cores` loop can't print or do other I/O: its order would change from run to run. | compile error |
| 4.9 | [on-cores-refuses-a-float-sum.kav](on-cores-refuses-a-float-sum.kav) | An `on cores` loop can't add floats into a shared `let`: the adding order would change the last bits. | compile error |
| 4.9 | [on-cores-refuses-a-shared-change.kav](on-cores-refuses-a-shared-change.kav) | A round of an `on cores` loop may change only its own slot: pushing to a shared list, or setting a shared variable, is a compile error. | compile error |
| 4.9 | [on-threads-rounds-may-print.kav](on-threads-rounds-may-print.kav) | An `on threads` loop's rounds may print and do other I/O, which `on cores` refuses; each round still writes only its own slot. | runs |
| 4.9 | [on-threads-rounds-run-at-once.kav](on-threads-rounds-run-at-once.kav) | `on threads(n)` rounds run at the same time as tasks: a round that waits less finishes first, while slots and sums are applied in round order. | runs |
| 4.9 | [on-threads-runs-at-most-n-rounds.kav](on-threads-runs-at-most-n-rounds.kav) | At most `n` rounds run at once: with `threads(1)` each ends before the next begins. | runs |
| 4.9 | [on-threads-leaving-early-runs-in-order.kav](on-threads-leaving-early-runs-in-order.kav) | A body that can leave early runs its rounds one after another. | runs |
| 4.9, 10.11 | [run-threads-caps-on-threads-rounds/](run-threads-caps-on-threads-rounds/src/main.kav) | `[run] threads = 1` caps the rounds running in the whole program: each ends before the next begins, whatever `threads(n)` asks. | runs |
| 4.9 | [on-cores-writes-the-enumerate-index-slot.kav](on-cores-writes-the-enumerate-index-slot.kav) | In `on cores for (i, x) in xs.enumerate()`, a round may write its own slot `out[i]` and gives the one-core result. | runs |
| 4.9 | [on-cores-writes-each-rounds-own-row.kav](on-cores-writes-each-rounds-own-row.kav) | An `on cores` round may write its own row of a grid kept in one list, `grid[i * w + j]` in `for j in 0..w` with a `w` that can't change, the parts in any order, and gives the one-core result. | runs |
| 4.9 | [on-cores-refuses-a-list-written-two-ways.kav](on-cores-refuses-a-list-written-two-ways.kav) | In an `on cores` loop a list written both as each round's slot and as its row is a compile error. | compile error |
| 4.9 | [on-cores-refuses-a-row-whose-width-can-change.kav](on-cores-refuses-a-row-whose-width-can-change.kav) | A row written in an `on cores` round needs a width that can't change; a `let` width is a compile error. | compile error |
| 4.9 | [on-before-a-value-gives-the-same-value.kav](on-before-a-value-gives-the-same-value.kav) | `on cores(n)`, `on cores` and `on cpu` go before a value, which is worked out there and is the same value; `on` stays a name elsewhere. | runs |
| 4.9 | [on-cores-before-a-value-count-of-zero-is-an-error.kav](on-cores-before-a-value-count-of-zero-is-an-error.kav) | `on cores(0)` before a value is a compile error. | compile error |
| 4.9 | [on-cores-before-a-value-count-under-one-panics.kav](on-cores-before-a-value-count-under-one-panics.kav) | A count under 1 worked out while running, before a value, panics. | panic |
| 4.9 | [on-cores-keeps-a-sum-modulo-a-number.kav](on-cores-keeps-a-sum-modulo-a-number.kav) | An `on cores` loop may keep a whole-number sum modulo a number written in the code, and gives the one-core result. | runs |
| 4.9 | [on-cores-count-under-one-panics.kav](on-cores-count-under-one-panics.kav) | A count worked out while running that is under 1 panics before the loop runs; more cores than there are uses them all. | panic |
| 4.9 | [on-cores-count-of-zero-is-an-error.kav](on-cores-count-of-zero-is-an-error.kav) | A count written as 0 or a negative number is a compile error: a loop runs on at least one core or thread. | compile error |
