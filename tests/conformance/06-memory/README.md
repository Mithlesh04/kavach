# 6. Memory

Conformance tests for spec chapter [6. Memory](../../../docs/spec/06-memory.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 6.1 | [lists-are-values.kav](lists-are-values.kav) | Assigning a list never shares it; changing the copy leaves the original alone. | runs |
| 6.1 | [passing-a-value-copies-it.kav](passing-a-value-copies-it.kav) | Passing a value to a function never shares it. | runs |
| 6.1 | [early-exits-free-values.kav](early-exits-free-values.kav) | Values made for an expression that leaves early (`??`, `try`, `return`, `continue`) are still freed. | runs |
| 6.2 | [shared-state-is-shared.kav](shared-state-is-shared.kav) | Shared<T> is shared: every copy sees the same value. | runs |
| 6.2 | [shared-updates-never-lose-one.kav](shared-updates-never-lose-one.kav) | `update` holds the `Shared` value's lock while its function runs, so updates from many tasks never lose one. | runs |
| 6.2 | [update-cant-wait.kav](update-cant-wait.kav) | The function given to `update` can't wait: a sleep, a channel, a task's `.wait()` or the network in it is a compile error. | compile error |
| 6.2 | [update-cant-wait-through-a-function.kav](update-cant-wait-through-a-function.kav) | A wait inside `update` reached through another function panics, since the value stays locked until the function returns. | panic |
| 6.2 | [shared-update-gives-a-result.kav](shared-update-gives-a-result.kav) | `update` gives the new value and `update_with` what its function returns beside it, both found under the lock; an `update` result may go unused. | runs |
| 6.3 | [arena-block-value.kav](arena-block-value.kav) | The value of an arena block is copied out of it. | runs |
| 6.3 | [arena-values-cannot-escape.kav](arena-values-cannot-escape.kav) | Storing an arena value outside the arena is a compile error. | compile error |
| 6.4 | [unsafe-needs-opt-in.kav](unsafe-needs-opt-in.kav) | A package may use `unsafe` only when kavach.toml sets unsafe = true (default: false). | compile error |
| 6.1 | [loop-writes-keep-value-rules.kav](loop-writes-keep-value-rules.kav) | Writing `list[i]` in a loop follows the value rules exactly: a copy made before keeps the old items. | runs |
| 6.1 | [drop-runs-at-the-end-of-the-block.kav](drop-runs-at-the-end-of-the-block.kav) | A value of a `Drop` type lives until the end of the block that holds it, even after its last use; `drop` runs there. | runs |
| 6.1 | [drop-runs-for-generic-types.kav](drop-runs-for-generic-types.kav) | A generic type can implement `Drop`: each instance's values run `drop`. | runs |
| 6.1 | [drop-runs-for-unions.kav](drop-runs-for-unions.kav) | A union can implement `Drop`: each of its values runs `drop`, whichever variant it is. | runs |
| 6.1 | [drop-runs-when-fail-leaves.kav](drop-runs-when-fail-leaves.kav) | When `fail` leaves blocks early, their `Drop` values drop then, the last made first. | runs |
| 6.1 | [drop-runs-at-the-end-of-each-round.kav](drop-runs-at-the-end-of-each-round.kav) | A `Drop` value made in a loop's body is dropped at the end of its own round, also out of a call that can fail. | runs |
| 6.1 | [drop-runs-for-what-a-failing-call-gave.kav](drop-runs-for-what-a-failing-call-gave.kav) | A `Drop` value a call that can fail gave, also kept in a `Shared` map, is dropped once its last holder lets it go. | runs |
| 6.1 | [drop-programs-return-from-catch.kav](drop-programs-return-from-catch.kav) | In a program with `Drop` types, a `catch` handler may return a value of any type from its function. | runs |
| 6.1 | [drop-programs-return-nothing-early.kav](drop-programs-return-nothing-early.kav) | In a program with `Drop` types, a function that can fail may `return` a call that gives nothing, also from `??` or a `match` arm. | runs |
| 6.1 | [drops-in-reverse-order.kav](drops-in-reverse-order.kav) | At the end of a block, the `Drop` values it made drop in the reverse order they were made. | runs |
| 6.1 | [drop-of-a-temporary.kav](drop-of-a-temporary.kav) | A `Drop` value made without a name drops at the end of its statement; a variable given a new value drops the old one there. | runs |
| 6.1 | [drop-given-to-underscore-warns.kav](drop-given-to-underscore-warns.kav) | A `Drop` value given to `_` drops at the end of its statement, with a warning to name it `_guard`. | warning |
| 6.1 | [drop-runs-once-for-copies.kav](drop-runs-once-for-copies.kav) | Copies of a `Drop` value are the same value: `drop` runs once, when the last holder ends; a changed copy is a value of its own. | runs |
| 6.1, 3.3 | [drop-waits-for-a-lambdas-block.kav](drop-waits-for-a-lambdas-block.kav) | A lambda holding a `Drop` value keeps it to the end of the lambda's block. | runs |
| 6.1, 2.15 | [drop-waits-for-an-interface-values-block.kav](drop-waits-for-an-interface-values-block.kav) | An interface value holding a `Drop` value keeps it to the end of its block. | runs |
| 6.1, 3.9 | [drop-waits-for-the-caller-after-inout.kav](drop-waits-for-the-caller-after-inout.kav) | An `inout` parameter given a new value: the old value drops when the call returns. | runs |
| 6.1 | [drop-needs-are-needed-where-made.kav](drop-needs-are-needed-where-made.kav) | The capabilities a type's `drop` uses are needed wherever a value of the type is made. | compile error |
| 6.1, 9.1 | [drop-is-not-run-after-a-panic.kav](drop-is-not-run-after-a-panic.kav) | A panic runs no `drop`: what a task that panics held is never dropped, and the program goes on. | runs |
| 6.2 | [a-shared-keeps-its-type.kav](a-shared-keeps-its-type.kav) | A `Shared<T>` can't be used as a `Shared<T?>`: its type can't change. | compile error |
| 6.4 | [pointers-need-unsafe.kav](pointers-need-unsafe.kav) | `alloc`, `free` and reading or writing through a `Ptr` are allowed only inside `unsafe`. | compile error |
| 6.4 | [deref-needs-a-pointer.kav](deref-needs-a-pointer.kav) | `*` reads through a pointer, so its value must be a `Ptr`. | compile error |
| 6.4 | [pointer-syntax-needs-unsafe.kav](pointer-syntax-needs-unsafe.kav) | `*p`, `p[i]` and `p + n` are allowed only inside `unsafe`. | compile error |
| 6.1, 2.7 | [drop-in-if-let-ends-with-its-block.kav](drop-in-if-let-ends-with-its-block.kav) | A `Drop` value bound by `if let` drops at the end of the `if let`'s block, before the next statement. | runs |
| 6.2 | [weak-does-not-keep-its-shared.kav](weak-does-not-keep-its-shared.kav) | A `Weak` gives back its `Shared` while it lives and `none` once it was freed; a `Shared` lives to the end of its block. | runs |
| 6.2 | [leaked-shared-cycles-are-reported.kav](leaked-shared-cycles-are-reported.kav) | A `Shared` value still alive when the program ends, which only a cycle can keep, is reported with the line that made it. | runs |
| 6.1, 6.2 | [drop-values-in-a-list.kav](drop-values-in-a-list.kav) | A `Drop` value in a list lives as long as the list holds it, and drops when the list ends, in a `Shared` too. | runs |
