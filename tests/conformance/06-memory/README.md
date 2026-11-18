# 6. Memory

Conformance tests for spec chapter [6. Memory](../../../docs/spec/06-memory.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 6.1 | [lists-are-values.kav](lists-are-values.kav) | Assigning a list never shares it; changing the copy leaves the original alone. | runs |
| 6.1 | [rows-made-from-one-list-are-separate.kav](rows-made-from-one-list-are-separate.kav) | A list built from the same list three times holds three separate rows: setting an item of one row changes only that row. | runs |
| 6.1 | [a-dict-keeps-the-value-it-was-given.kav](a-dict-keeps-the-value-it-was-given.kav) | A dict holds the value it was given: changing the variable afterwards leaves the dict alone, and the other way round. | runs |
| 6.1 | [passing-a-value-copies-it.kav](passing-a-value-copies-it.kav) | Passing a value to a function never shares it. | runs |
| 6.1 | [text-grows-in-place-copies-stay.kav](text-grows-in-place-copies-stay.kav) | Text with one holder grows in place under `+=`; a copy taken before keeps its old value. | runs |
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
| 6.1 | [a-list-in-a-field-grows-in-place.kav](a-list-in-a-field-grows-in-place.kav) | `push` onto a list in a field or an item changes it where it is; a copy made before keeps its items, and the pushed value may read the same list. | runs |
| 6.1 | [text-items-copied-between-lists.kav](text-items-copied-between-lists.kav) | Text pushed from one list into another, short or long, is a copy: changing either list leaves the other as it was. | runs |
| 6.1 | [text-items-assigned-between-lists.kav](text-items-assigned-between-lists.kav) | `xs[i] = ys[j]` on lists of text copies the item: a copy of `xs` made before, and `ys`, keep theirs. | runs |
| 6.1 | [text-items-set-from-their-own-list.kav](text-items-set-from-their-own-list.kav) | An item of a list of text set from the same list, itself included, gives what a copy would; a copy made before keeps its items. | runs |
| 6.1 | [a-map-in-a-field-changes-in-place.kav](a-map-in-a-field-changes-in-place.kav) | `r.names[k] = v` changes the map in the field where it is, directly or through `inout`; a copy made before keeps its entries. | runs |
| 6.1 | [drop-runs-once-for-copies-on-other-cores.kav](drop-runs-once-for-copies-on-other-cores.kav) | `drop` runs once, for the last holder, when copies go to tasks on other cores or come out of a `Shared` there. | runs |
| 6.1 | [a-copy-in-a-tuple-or-optional-is-its-own.kav](a-copy-in-a-tuple-or-optional-is-its-own.kav) | A value copied into a tuple, an optional or a union variant is its own: changing the variable later doesn't change them. | runs |
| 6.1 | [a-function-changing-its-copy-leaves-the-callers-record.kav](a-function-changing-its-copy-leaves-the-callers-record.kav) | A function that copies a record argument into a `let` and changes its nested list and field changes only its copy; the caller's value is unchanged. | runs |
| 6.1 | [a-list-copied-into-and-out-of-a-list-is-its-own.kav](a-list-copied-into-and-out-of-a-list-is-its-own.kav) | A list put into another list, or taken out of one into a `let`, is a copy: changing either side leaves the other alone. | runs |
| 6.1 | [a-list-pushed-each-round-keeps-that-rounds-copy.kav](a-list-pushed-each-round-keeps-that-rounds-copy.kav) | A `let` pushed into a list each round of a loop, and changed after, gives a copy as it was at each push. | runs |
| 6.1 | [a-map-or-set-copy-is-its-own.kav](a-map-or-set-copy-is-its-own.kav) | Copying a map or a set never shares it: inserting, setting a key or removing on one side leaves the other alone. | runs |
| 6.1, 9.4 | [a-receiver-lives-to-its-blocks-end.kav](a-receiver-lives-to-its-blocks-end.kav) | A channel's ends live until the end of the block that holds them, as a `Shared` does: a receiver never used again still counts until then. | runs |
| 6.1 | [a-record-copy-is-its-own-to-any-depth.kav](a-record-copy-is-its-own-to-any-depth.kav) | A record copied into another `let`, out of a list, or into another record's field is its own value, down to the lists and records inside it. | runs |
| 6.1, 3.9 | [in-place-changes-leave-earlier-copies.kav](in-place-changes-leave-earlier-copies.kav) | A list changed in place through `inout` or as an item of a list leaves every copy taken before it as it was. | runs |
| 6.2, 1.4 | [a-const-shared-cannot-be-replaced.kav](a-const-shared-cannot-be-replaced.kav) | a `Shared` named with `const` can be updated, but the name can't be given another `Shared`. | compile error |
| 6.2, 1.4 | [a-let-given-a-new-shared-leaves-its-copies.kav](a-let-given-a-new-shared-leaves-its-copies.kav) | copies of a `Shared` see one value; a `let` given a new `Shared` names the new one, and the copies still share the old. | runs |
| 6.2, 9.5 | [a-shared-inside-a-record-or-list-is-shared.kav](a-shared-inside-a-record-or-list-is-shared.kav) | a `Shared` kept in a record, a list or a lambda is still the one value: copying the record or list copies the link, not the value, so tasks and lambdas update what everyone sees. | runs |
| 6.1, 6.2 | [a-shared-inside-a-dict-is-shared.kav](a-shared-inside-a-dict-is-shared.kav) | A `Shared` put in a dict is the same value: a change through the variable shows through the dict. | runs |
| 6.2 | [a-value-holding-a-shared-lives-to-its-blocks-end.kav](a-value-holding-a-shared-lives-to-its-blocks-end.kav) | A value that holds a `Shared` lives until the end of the block that holds it, after its last use; then the `Shared` is freed. | runs |
| 6.2 | [a-weak-back-link-leaks-nothing.kav](a-weak-back-link-leaks-nothing.kav) | Cycles form only through `Shared`: a back-link made a `Weak` forms none, so nothing is reported as leaked when the program ends. | runs |
| 6.2 | [a-weak-gives-back-the-same-shared.kav](a-weak-gives-back-the-same-shared.kav) | `w.shared()` gives back the very `Shared` the `Weak` points to, wherever the `Weak` is kept, until the block that holds the `Shared` ends; then it gives `none`. | runs |
| 6.2 | [update-cant-take-from-a-channel.kav](update-cant-take-from-a-channel.kav) | the function given to `update` can't wait on a channel: a `take` in it is a compile error. | compile error |
| 6.2 | [update-cant-wait-for-a-task.kav](update-cant-wait-for-a-task.kav) | the function given to `update` can't wait for a task: a `.wait()` in it is a compile error. | compile error |
| 6.2, 9.4 | [update-in-a-task-cant-put-into-a-channel.kav](update-in-a-task-cant-put-into-a-channel.kav) | inside a task too, the function given to `update` can't wait: a channel `put` in it is a compile error. | compile error |
| 6.2, 9.7 | [update-results-come-back-from-tasks.kav](update-results-come-back-from-tasks.kav) | `update` gives the new value read under the lock, and `update_with` the result its function gives, so tasks that update one `Shared` each get their own, distinct result. | runs |
| 6.3 | [arena-as-a-statement.kav](arena-as-a-statement.kav) | An arena block may stand as a statement; its last line may be a call that gives nothing, such as `print`. | runs |
| 6.3, 6.1 | [arena-drop-values-go-when-it-ends.kav](arena-drop-values-go-when-it-ends.kav) | Values made in an arena are freed when it ends: `Drop` values made there drop at its end, last made first, before the code after it runs. | runs |
| 6.3 | [arena-in-a-loop-gives-each-rounds-value.kav](arena-in-a-loop-gives-each-rounds-value.kav) | An arena in a loop starts afresh each round, and each round's block value comes out of it. | runs |
| 6.3 | [arena-inner-values-cannot-escape-into-the-outer-arena.kav](arena-inner-values-cannot-escape-into-the-outer-arena.kav) | A nested arena is an arena too: its values can't be stored in a variable of the arena around it. | compile error |
| 6.3, 9.1 | [arena-inside-a-task.kav](arena-inside-a-task.kav) | A task may use an arena of its own; the arena's value is the task's result. | runs |
| 6.3 | [arena-made-value-cannot-escape-whatever-made-it.kav](arena-made-value-cannot-escape-whatever-made-it.kav) | A value made in an arena can't be stored outside it however it was made: by a call, not only as a named value. | compile error |
| 6.3 | [arena-outer-values-cannot-escape-from-an-inner-arena.kav](arena-outer-values-cannot-escape-from-an-inner-arena.kav) | A value made in an outer arena can't be stored, from an inner arena, in a variable declared before the outer one. | compile error |
| 6.3, 6.2 | [arena-shared-is-freed-when-it-ends.kav](arena-shared-is-freed-when-it-ends.kav) | A `Shared` made in an arena is freed when the arena ends: a `Weak` to it, copied out as the block's value, then gives `none`. | runs |
| 6.3, 9.2 | [arena-tasks-finish-before-it-ends.kav](arena-tasks-finish-before-it-ends.kav) | A task spawned inside an arena finishes before the arena ends, so it never sees the arena's values freed. | runs |
| 6.3 | [arena-tasks-may-use-its-values.kav](arena-tasks-may-use-its-values.kav) | A task spawned inside an arena may use the arena's values. | runs |
| 6.3, 3.3 | [arena-value-cannot-be-captured-by-an-outside-lambda.kav](arena-value-cannot-be-captured-by-an-outside-lambda.kav) | A lambda that uses an arena value can't be stored in a variable declared before the arena. | compile error |
| 6.3 | [arena-value-cannot-be-captured-by-an-outside-task.kav](arena-value-cannot-be-captured-by-an-outside-task.kav) | A task that uses an arena value can't be stored in a variable declared before the arena. | compile error |
| 6.3 | [arena-value-cannot-be-given-to-an-outside-let.kav](arena-value-cannot-be-given-to-an-outside-let.kav) | Giving a variable declared before an arena a value made in the arena is a compile error, also in a loop. | compile error |
| 6.3, 9.4 | [arena-value-cannot-be-put-into-an-outside-channel.kav](arena-value-cannot-be-put-into-an-outside-channel.kav) | Putting an arena value into a channel made before the arena is a compile error. | compile error |
| 6.3 | [arena-value-cannot-be-returned.kav](arena-value-cannot-be-returned.kav) | The only way out of an arena is its block's value: `return` of a value made in the arena is a compile error. | compile error |
| 6.3 | [arena-value-cannot-be-stored-in-an-outside-field.kav](arena-value-cannot-be-stored-in-an-outside-field.kav) | Storing an arena value in a field of a record declared before the arena is a compile error. | compile error |
| 6.3 | [arena-value-cannot-be-stored-in-an-outside-map.kav](arena-value-cannot-be-stored-in-an-outside-map.kav) | Storing an arena value in a map declared before the arena is a compile error. | compile error |
| 6.3, 6.2 | [arena-value-cannot-be-stored-in-an-outside-shared.kav](arena-value-cannot-be-stored-in-an-outside-shared.kav) | Storing an arena value in a `Shared` declared before the arena is a compile error. | compile error |
| 6.3, 3.9 | [arena-value-cannot-leave-through-inout.kav](arena-value-cannot-leave-through-inout.kav) | Passing a variable declared before an arena as an `inout` argument, with an arena value to store in it, is a compile error. | compile error |
| 6.3, 6.1 | [arena-value-is-copied-out-whole.kav](arena-value-is-copied-out-whole.kav) | An arena block's value may be a record that holds values made in the arena; it is copied out whole, and a copy of it changes on its own. | runs |
| 6.3, 3.3 | [arena-value-may-be-a-lambda-using-its-values.kav](arena-value-may-be-a-lambda-using-its-values.kav) | An arena block's value may be a lambda that uses values made in the arena: the lambda, with what it holds, is copied out. | runs |
| 6.3 | [arenas-nest.kav](arenas-nest.kav) | An arena may be nested in another: the inner one may use the outer one's values, and its value is copied out into the outer one. | runs |
| 6.4 | [a-pointer-points-only-at-c-types.kav](a-pointer-points-only-at-c-types.kav) | A `Ptr<T>` points at values C passes as they are: `T` is a number, a `Bool`, a `Ptr` or a record of only those, so `Ptr<String>` is a compile error. | compile error |
| 6.4 | [free-needs-unsafe.kav](free-needs-unsafe.kav) | `free` gives raw memory back, so it is allowed only inside `unsafe { … }`. | compile error |
| 6.4 | [pointer-arithmetic-needs-unsafe.kav](pointer-arithmetic-needs-unsafe.kav) | `p + n` and `p - n` are raw pointer arithmetic, so they are allowed only inside `unsafe { … }`. | compile error |
| 6.4 | [pointer-bytes-need-unsafe.kav](pointer-bytes-need-unsafe.kav) | `p.bytes(n)` copies bytes out of raw memory, so it is allowed only inside `unsafe { … }`. | compile error |
| 6.4 | [pointer-read-needs-unsafe.kav](pointer-read-needs-unsafe.kav) | `p.read(i)` reads raw memory, so it is allowed only inside `unsafe { … }`. | compile error |
| 6.4 | [pointer-text-needs-unsafe.kav](pointer-text-needs-unsafe.kav) | `p.text()` copies text out of raw memory, so it is allowed only inside `unsafe { … }`. | compile error |
| 6.4 | [pointer-write-needs-unsafe.kav](pointer-write-needs-unsafe.kav) | `p.write(i, v)` writes raw memory, so it is allowed only inside `unsafe { … }`. | compile error |
| 6.4 | [ptr-from-int-needs-unsafe.kav](ptr-from-int-needs-unsafe.kav) | `Ptr.from_int(n)` makes a raw address, so it is allowed only inside `unsafe { … }`. | compile error |
| 6.3 | [arena-copied-parts-may-leave.kav](arena-copied-parts-may-leave.kav) | A number, `Bool` or `Char` worked out from an arena's values is copied, so text built from one may leave the arena. | runs |
