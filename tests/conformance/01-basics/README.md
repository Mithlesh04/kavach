# 1. Basics

Conformance tests for spec chapter [1. Basics](../../../docs/spec/01-basics.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 1.1 | [line-continuation.kav](line-continuation.kav) | A statement continues when a line ends with an operator or the next line starts with `.`. | runs |
| 1.1 | [a-byte-order-mark-is-a-space.kav](a-byte-order-mark-is-a-space.kav) | A byte order mark at the start of a file counts as a space. | runs |
| 1.1 | [no-semicolons.kav](no-semicolons.kav) | Statements end at the end of a line; there are no semicolons. | compile error |
| 1.2 | [block-comments-nest.kav](block-comments-nest.kav) | `/* … */` comments run over lines, nest, and sit between tokens. | runs |
| 1.2 | [block-comments-must-close.kav](block-comments-must-close.kav) | A `/*` comment never closed is an error. | compile error |
| 1.3 | [main-is-entry-point.kav](main-is-entry-point.kav) | A program starts at fn main(). | runs |
| 1.3 | [top-level-statements.kav](top-level-statements.kav) | An entry file may contain top-level statements; they become the body of main. | runs |
| 1.4 | [declarations-can-come-later.kav](declarations-can-come-later.kav) | Top-level declarations can be used before they appear in the file. | runs |
| 1.4 | [inner-scope-reuses-name.kav](inner-scope-reuses-name.kav) | An inner scope may reuse a name from an outer scope. | runs |
| 1.4 | [hiding-let-that-is-never-used-warns.kav](hiding-let-that-is-never-used-warns.kav) | A never-used variable that hides an outer `let` gets a warning that says so. | warning |
| 1.4 | [let-cannot-be-reassigned.kav](let-cannot-be-reassigned.kav) | A `const` binding cannot be reassigned. | compile error |
| 1.4 | [let-mut-is-not-kavach.kav](let-mut-is-not-kavach.kav) | `let mut` is a compile error that says to write `let`. | compile error |
| 1.7 | [triple-quotes-are-not-kavach.kav](triple-quotes-are-not-kavach.kav) | `"""…"""` is a compile error that says to write text over lines in backticks, with `\{`, `` \` `` and `\\`. | compile error |
| 1.7 | [a-template-on-its-backticks-line-stays-there.kav](a-template-on-its-backticks-line-stays-there.kav) | A template whose text starts on its backtick's line can't go on to the next line: a compile error that says to start the text on the next line. | compile error |
| 1.7 | [quoted-text-stays-on-one-line.kav](quoted-text-stays-on-one-line.kav) | Text in quotes stays on one line: a line break before its closing quote is a compile error. | compile error |
| 1.7 | [quoted-text-is-the-characters-written.kav](quoted-text-is-the-characters-written.kav) | `"\d{3}"` is the 5 characters between its quotes. | runs |
| 1.7 | [a-template-over-lines-drops-common-indentation.kav](a-template-over-lines-drops-common-indentation.kav) | A template over lines loses the common indentation; deeper lines keep the rest of theirs. | runs |
| 1.7 | [a-template-over-lines-keeps-empty-lines.kav](a-template-over-lines-keeps-empty-lines.kav) | Empty lines stay, and a line of only spaces doesn't count for the common indentation. | runs |
| 1.7 | [a-template-line-of-only-a-value.kav](a-template-line-of-only-a-value.kav) | A line of a template over lines may hold only a `{…}`. | runs |
| 1.7 | [json-over-lines-in-a-template.kav](json-over-lines-in-a-template.kav) | JSON over lines is written with `\{` for a brace, and decodes through `json`. | runs |
| 1.7 | [a-template-over-lines-reads-escapes.kav](a-template-over-lines-reads-escapes.kav) | A template over lines reads `` \` ``, `\\` and `\{`. | runs |
| 1.7 | [quotes-in-a-template-over-lines.kav](quotes-in-a-template-over-lines.kav) | `"` and `'` in a template over lines need no escape. | runs |
| 1.7 | [a-template-over-lines-is-an-argument-and-a-result.kav](a-template-over-lines-is-an-argument-and-a-result.kav) | A template over lines can be given to a function and returned from one. | runs |
| 1.7 | [f-strings-are-not-kavach.kav](f-strings-are-not-kavach.kav) | `f"…"` is a compile error that says a template is written in backticks. | compile error |
| 1.4 | [locals-only-after-let.kav](locals-only-after-let.kav) | A local variable can be used only after its `const`. | compile error |
| 1.4 | [no-duplicate-name-in-scope.kav](no-duplicate-name-in-scope.kav) | A name must not be declared twice in the same scope. | compile error |
| 1.4 | [no-overloading.kav](no-overloading.kav) | Two functions can't share a name: Kavach has no overloading. | compile error |
| 1.4 | [unknown-name.kav](unknown-name.kav) | Using a name that was never declared is a compile error. | compile error |
| 1.4 | [var-can-change.kav](var-can-change.kav) | A `let` binding can be changed. | runs |
| 1.5 | [types-use-pascal-case.kav](types-use-pascal-case.kav) | Type names must be PascalCase. | compile error |
| 1.5 | [unicode-names.kav](unicode-names.kav) | Names may use letters from any Unicode script. | runs |
| 1.5 | [variables-use-snake-case.kav](variables-use-snake-case.kav) | Variable names must be snake_case. | compile error |
| 1.5 | [capital-names-in-a-const-are-variants.kav](capital-names-in-a-const-are-variants.kav) | A capitalized name in a pattern is a variant: `const COUNT = 10` is an error saying to write `count`. | compile error |
| 1.7 | [decimal-literal-from-context.kav](decimal-literal-from-context.kav) | A number literal takes its type from context, so a Decimal literal is exact. | runs |
| 1.7 | [integer-literals.kav](integer-literals.kav) | Integer literals may use `_` separators, hex (0x), octal (0o) and binary (0b). | runs |
| 1.7 | [multi-line-string.kav](multi-line-string.kav) | Text over lines leaves out the line break after its opening backtick, and the one before a closing backtick on its own line. | runs |
| 1.7 | [a-quoted-string-is-plain.kav](a-quoted-string-is-plain.kav) | A string in quotes has no escapes and fills nothing in. | runs |
| 1.7 | [a-quoted-string-that-looks-like-a-template-warns.kav](a-quoted-string-that-looks-like-a-template-warns.kav) | `"Hello, {name}"` with `name` in scope, or `"a\nb"`, warns to use backticks; a path or a pattern doesn't. | warning |
| 1.7 | [r-strings-are-not-kavach.kav](r-strings-are-not-kavach.kav) | `r"…"` is a compile error that says to drop the `r`. | compile error |
| 1.7 | [a-template-spans-lines.kav](a-template-spans-lines.kav) | A template whose text starts on the next line spans lines, and fills in `{…}` there. | runs |
| 1.7 | [string-escapes.kav](string-escapes.kav) | Templates read the escapes \` \\ \{ and \u{…}; a `"` needs none. | runs |
| 1.7 | [string-interpolation.kav](string-interpolation.kav) | Any expression inside {} in a template is inserted; \{ is a literal brace. | runs |
| 1.7 | [interpolation-closes-on-its-line.kav](interpolation-closes-on-its-line.kav) | In a one-line template, a `{` must close on its line: `` `{` `` is one error at the `{`. | compile error |
| 1.7 | [interpolation-of-only-text-warns.kav](interpolation-of-only-text-warns.kav) | A `{…}` holding only text warns: `` t == `{" or t == "}` `` is a single template. | warning |
| 1.7 | [negative-literals-reach-the-minimum.kav](negative-literals-reach-the-minimum.kav) | A minus before a whole-number literal belongs to it: `-128` is an `Int8`. | runs |
| 1.8 | [assignment-is-a-statement.kav](assignment-is-a-statement.kav) | Assignment is a statement, not an expression. | compile error |
| 1.8 | [integer-division-truncates.kav](integer-division-truncates.kav) | Int / Int is an Int, truncated. | runs |
| 1.8 | [remainders-of-a-loop-counter.kav](remainders-of-a-loop-counter.kav) | `i % n` of a counted loop's counter is its remainder in every round, from any start, with `continue`, `break` and nested loops. | runs |
| 1.8 | [operator-precedence.kav](operator-precedence.kav) | Operators follow the precedence table: * before +, not before and, ?? lowest. | runs |
| 1.8 | [equality-compares-contents.kav](equality-compares-contents.kav) | `==` compares contents at every depth; maps and sets ignore order, lists keep it. | runs |
| 1.8 | [comparisons-chain.kav](comparisons-chain.kav) | `a < b < c` is `a < b and b < c`, each operand once, stopping at the first false pair. | runs |
| 1.8, 14.2 | [values-with-an-order.kav](values-with-an-order.kav) | Numbers, text, characters, `Bool` and tuples have an order: text by code point, `false` before `true`, tuples item by item; `sort_by` takes such a key. | runs |
| 1.8 | [records-have-no-order.kav](records-have-no-order.kav) | A record has no order: `<` on two records is a compile error. | compile error |
| 1.8 | [power.kav](power.kav) | `a ** b` is a power: right to left, tighter than a minus on its left. | runs |
| 1.8, 5.6 | [power-overflow-panics.kav](power-overflow-panics.kav) | A whole-number power that doesn't fit its type panics. | panic |
| 1.8 | [power-negative-exponent-panics.kav](power-negative-exponent-panics.kav) | A whole number to a negative power panics. | panic |
| 1.8 | [compound-assignment-for-bits.kav](compound-assignment-for-bits.kav) | `&= \|= ^= <<= >>=` change a variable as `x = x op v` would. | runs |
| 1.8 | [default-groups-to-the-right.kav](default-groups-to-the-right.kav) | `a ?? b ?? c` is `a ?? (b ?? c)`: the first value present wins, and `b` may be an optional too. | runs |
| 1.8 | [compound-assignment-checks-its-operator.kav](compound-assignment-checks-its-operator.kav) | `x op= v` is checked as `x op v` is. | compile error |
| 1.8 | [shifts-move-bits.kav](shifts-move-bits.kav) | `<<` and `>>` shift bits; `>>` keeps the sign. | runs |
| 1.8, 5.6 | [shift-overflow-panics.kav](shift-overflow-panics.kav) | `a << n` panics when the result doesn't fit. | panic |
| 1.8, 5.6 | [shift-by-width-panics.kav](shift-by-width-panics.kav) | Shifting by a negative amount or by the width or more panics. | panic |
| 1.8 | [a-known-loop-that-always-panics-is-an-error.kav](a-known-loop-that-always-panics-is-an-error.kav) | In a `for` over known numbers that can't leave early, a statement that always runs is checked for the loop's values: an index past the end. | compile error |
| 1.8 | [a-known-loop-that-divides-by-zero-is-an-error.kav](a-known-loop-that-divides-by-zero-is-an-error.kav) | A `for` over known numbers whose first round divides by 0 is a compile error. | compile error |
| 1.8, 7.7 | [a-known-loop-that-breaks-a-require-is-an-error.kav](a-known-loop-that-breaks-a-require-is-an-error.kav) | A `for` over known numbers that passes a function a value its `require` refuses is a compile error, naming the first such value. | compile error |
| 1.8 | [a-known-loop-that-adds-past-its-type-is-an-error.kav](a-known-loop-that-adds-past-its-type-is-an-error.kav) | `t += c` in a `for` over known numbers that carries `t` past its type is a compile error, naming the round that does. | compile error |
| 1.8 | [a-known-loop-that-takes-past-its-type-is-an-error.kav](a-known-loop-that-takes-past-its-type-is-an-error.kav) | `t -= c` in a `for` over known numbers that carries `t` below its type is a compile error, naming the round that does. | compile error |
| 1.8 | [a-known-value-through-a-conversion-is-checked.kav](a-known-value-through-a-conversion-is-checked.kav) | A known whole number converted to a whole-number type it fits stays known: `Int8(i) * 2` in a `for` over known numbers is a compile error at the first round that overflows. | compile error |
| 1.8 | [a-loop-that-may-not-reach-a-round-is-not-assumed.kav](a-loop-that-may-not-reach-a-round-is-not-assumed.kav) | A `for` that can leave early, a statement in an `if`, an inner loop that may not run, or a range that fits gets no compile error. | runs |
| 1.8 | [a-let-that-always-overflows-is-an-error.kav](a-let-that-always-overflows-is-an-error.kav) | A `let` set to known values, with nothing between that may change it, is checked as a `const` is. | compile error |
| 1.8 | [a-let-a-branch-or-loop-may-change-is-not-assumed.kav](a-let-a-branch-or-loop-may-change-is-not-assumed.kav) | A `let` a branch, a loop or a lambda may set, an `if` tests, or an `inout` call may change, is not assumed known. | runs |
| 1.8 | [arithmetic-that-always-panics-is-an-error.kav](arithmetic-that-always-panics-is-an-error.kav) | `+ - *` of known values whose result doesn't fit the type is a compile error. | compile error |
| 1.8, 2.7 | [unwrap-of-a-known-none-is-an-error.kav](unwrap-of-a-known-none-is-an-error.kav) | `.unwrap()` on a value the compiler knows is `none` always panics, so it is a compile error. | compile error |
| 1.8 | [unwrap-of-a-none-that-may-change-is-not-assumed.kav](unwrap-of-a-none-that-may-change-is-not-assumed.kav) | An optional set to `none` is known no more after a branch or a loop that may set it, nor where an `if` tests it, so `.unwrap()` there gets no compile error. | runs |
| 1.8 | [division-by-a-known-zero-is-an-error.kav](division-by-a-known-zero-is-an-error.kav) | A whole-number division or remainder by a known `0` is a compile error. | compile error |
| 1.8 | [index-past-a-known-end-is-an-error.kav](index-past-a-known-end-is-an-error.kav) | An index outside a list whose length is known is a compile error. | compile error |
| 1.8 | [uneven-whole-literal-division-warns.kav](uneven-whole-literal-division-warns.kav) | `/` between two whole-number literals that don't divide evenly gives a warning. | warning |
| 1.8 | [logic-is-written-in-words.kav](logic-is-written-in-words.kav) | `!ready` is a compile error that says to write `not`. | compile error |
| 1.8 | [lists-bytes-and-rings-have-an-order.kav](lists-bytes-and-rings-have-an-order.kav) | Lists, rings and `Bytes` compare item by item, the shorter first when one runs out; lists of lists sort. | runs |
| 1.9 | [blocks-are-expressions.kav](blocks-are-expressions.kav) | A block has the value of its last expression. | runs |
| 1.11 | [values-print-like-literals.kav](values-print-like-literals.kav) | print shows values the way they are written in code. | runs |
| 1.12 | [destructuring-lists.kav](destructuring-lists.kav) | Lists come apart by position: missing items are `none` or their default; `...rest`; `_`. | runs |
| 1.12 | [destructuring-records.kav](destructuring-records.kav) | Records come apart by field name, renamed or with a default for a `T?` field. | runs |
| 1.12 | [destructuring-maps.kav](destructuring-maps.kav) | Maps come apart by key; a bare name is a text key. | runs |
| 1.12 | [destructuring-nests.kav](destructuring-nests.kav) | Patterns nest; inside a part that may be missing, names are `none` unless they have defaults. | runs |
| 1.12 | [destructuring-needs-the-field.kav](destructuring-needs-the-field.kav) | A record pattern names only fields the record has. | compile error |
| 1.12, 8.7 | [destructuring-keeps-secrets.kav](destructuring-keeps-secrets.kav) | A name taken from a `secret` field is a secret. | compile error |
| 1.12 | [list-patterns-are-not-for-match.kav](list-patterns-are-not-for-match.kav) | List and record patterns go in `const` and `let`, not `match`. | compile error |
| 1.13 | [module-constants-are-read-by-functions.kav](module-constants-are-read-by-functions.kav) | A top-level `const` is a module constant every function of the file reads, worked out before `main`. | runs |
| 1.13 | [module-constants-use-no-capability.kav](module-constants-use-no-capability.kav) | A module constant can't use a capability. | compile error |
| 1.13 | [module-constant-panics-before-main.kav](module-constant-panics-before-main.kav) | A module constant that panics stops the program before `main`. | panic |
| 1.13, 1.3 | [script-constants-may-use-capabilities.kav](script-constants-may-use-capabilities.kav) | A script's `const` that uses a capability is one of its statements; plain constants stay module constants. | runs |
| 1.13 | [functions-read-no-script-statement.kav](functions-read-no-script-statement.kav) | A function can't read a script's `const` that uses a capability. | compile error |
| 1.13, 1.4, 10.1 | [unused-pub-constants-are-warned.kav](unused-pub-constants-are-warned.kav) | A module constant nothing reads gets a warning, a `pub` one too when nothing in its package reads it. | warning |
| 1.3 | [statements-outside-main-are-an-error.kav](statements-outside-main-are-an-error.kav) | In a file with `fn main`, a statement outside `main` is a compile error, as it would never run; a top-level `const` is still a module constant. | compile error |
| 1.3 | [main-takes-no-parameters.kav](main-takes-no-parameters.kav) | `main` takes no parameters: a program reads its arguments with `cli.args()`. | compile error |
| 1.11 | [text-in-collections-shows-as-written.kav](text-in-collections-shows-as-written.kav) | Text inside a collection shows as written, combining marks too; only quotes, backslashes, line breaks, tabs, and control and invisible characters are escaped. | runs |
| 1.7 | [a-carriage-return-escape.kav](a-carriage-return-escape.kav) | `\r` in a template is a carriage return, one character, as protocols ending lines with `\r\n` need. | runs |
| 1.8 | [sets-and-maps-have-no-order.kav](sets-and-maps-have-no-order.kav) | A set or a map has no order, so comparing two with `<` is a compile error. | compile error |
| 1.11 | [collections-print-like-literals.kav](collections-print-like-literals.kav) | Collections print as they are written: text items quoted, nested items in their own form, an empty list `[]`, an empty map `[:]`. | runs |
| 1.11 | [decimals-print-every-digit-they-keep.kav](decimals-print-every-digit-they-keep.kav) | A `Decimal` prints all its digits, keeping its scale, alone, in text and inside collections and records. | runs |
| 1.12 | [const-tuple-names-cannot-change.kav](const-tuple-names-cannot-change.kav) | Names taken from a tuple with `const` never change; with `let` they are variables. | compile error |
| 1.4 | [a-hiding-let-warns-in-every-scope.kav](a-hiding-let-warns-in-every-scope.kav) | A never-used variable that hides an outer `let` of the same name gets the hiding warning in any inner scope: `if`, `while`, a plain block and a lambda; one that is used gets none. | warning |
| 1.4 | [a-hiding-name-is-made-from-the-outer-one.kav](a-hiding-name-is-made-from-the-outer-one.kav) | A local can be used only after its `const` or `let`, so the value that makes a hiding name reads the outer one; changing the outer name inside a scope, without `let`, changes it for good. | runs |
| 1.4, 1.12 | [a-hiding-pattern-name-warns.kav](a-hiding-pattern-name-warns.kav) | The hiding warning covers every kind of never-used variable: a name a pattern takes, and a `const`, that hide an outer `let` get it too. | warning |
| 1.4, 1.5 | [a-long-name-given-twice-is-refused.kav](a-long-name-given-twice-is-refused.kav) | A name declared twice in one scope is refused however long it is. | compile error |
| 1.4 | [a-name-cannot-read-itself.kav](a-name-cannot-read-itself.kav) | A local can be used only after its `const` or `let`, so its own value can't read it. | compile error |
| 1.4 | [a-name-ends-with-its-block.kav](a-name-ends-with-its-block.kav) | A name declared in a block can't be used after the block: it is not declared there. | compile error |
| 1.4, 1.9 | [every-inner-scope-may-reuse-a-name.kav](every-inner-scope-may-reuse-a-name.kav) | Every inner scope (a plain block, `if`, `else`, `while`, `for`, a `match` arm, a lambda) may reuse an outer name; the outer name keeps its value after it. | runs |
| 1.4, 4.2 | [if-let-names-never-change.kav](if-let-names-never-change.kav) | The name `if let` binds never changes: assigning to it is a compile error. | compile error |
| 1.4, 1.5 | [long-names-are-compared-in-full.kav](long-names-are-compared-in-full.kav) | Two names that differ only in their last character are two names, however long, for `const`, `let`, fields and functions. | runs |
| 1.4, 1.12 | [no-duplicate-name-across-pattern-and-let.kav](no-duplicate-name-across-pattern-and-let.kav) | A name a pattern took can't be declared again by a `let` in the same scope. | compile error |
| 1.4, 1.12 | [no-duplicate-name-in-one-pattern.kav](no-duplicate-name-in-one-pattern.kav) | One pattern must not take the same name twice: it would declare it twice in one scope. | compile error |
| 1.4, 1.12 | [underscore-names-are-unused-on-purpose.kav](underscore-names-are-unused-on-purpose.kav) | A name starting with `_` (or `_` alone) is unused on purpose: it gets no warning in any binding place, while a plain unused name beside it does. | warning |
| 1.4, 1.12 | [unused-pattern-names-warn.kav](unused-pattern-names-warn.kav) | A name a pattern takes (from a list, a record, a map, a tuple, a nested pattern, or `...rest`) that is never used gets a warning, while the names used beside it get none. | warning |
| 1.4 | [unused-variables-warn.kav](unused-variables-warn.kav) | A `const`, a `let`, a `for` variable or a `catch` name that is never used gets a warning, and the program still runs. | warning |
| 1.4 | [unused-parameters-warn.kav](unused-parameters-warn.kav) | A function or method parameter that the function never reads gets a warning, and the program still runs. | warning |
| 1.4 | [underscore-parameters-are-unused-on-purpose.kav](underscore-parameters-are-unused-on-purpose.kav) | A parameter named `_` or starting with `_` gets no warning, while a plain unused one beside it does. | warning |
| 1.4, 2.8 | [interface-methods-keep-unused-parameters.kav](interface-methods-keep-unused-parameters.kav) | A method that implements an interface gets no warning for a parameter it never reads; another method of the same type does. | warning |
| 1.4, 2.16 | [operator-methods-keep-unused-parameters.kav](operator-methods-keep-unused-parameters.kav) | A method that implements an operator interface gets no warning for a parameter it never reads. | warning |
| 1.4, 3.4 | [a-function-used-as-a-value-keeps-unused-parameters.kav](a-function-used-as-a-value-keeps-unused-parameters.kav) | A function used as a value gets no warning for a parameter it never reads; a function only called does. | warning |
| 1.4, 3.3 | [a-parameter-read-in-a-lambda-is-used.kav](a-parameter-read-in-a-lambda-is-used.kav) | A parameter read only inside a lambda in the body is used. | warning |
| 1.4, 7.1, 7.2 | [a-parameter-read-in-a-contract-is-used.kav](a-parameter-read-in-a-contract-is-used.kav) | A parameter read only in a `require` or an `ensure` is used. | warning |
| 1.4, 10.15 | [a-literal-check-function-keeps-unused-parameters.kav](a-literal-check-function-keeps-unused-parameters.kav) | A function a `literal` names as its `check` or `param` gets no warning for a parameter it never reads. | warning |
| 1.4, 8.3 | [agent-handlers-keep-unused-parameters.kav](agent-handlers-keep-unused-parameters.kav) | An agent's `on` handler gets no warning for a parameter it never reads; a plain function does. | warning |
| 1.4, 15.1 | [gpu-functions-keep-unused-parameters.kav](gpu-functions-keep-unused-parameters.kav) | A `gpu fn` gets no warning for a parameter it never reads. | warning |
| 1.4, 1.6 | [var-is-not-kavach.kav](var-is-not-kavach.kav) | `var` is not Kavach: writing it is a compile error that says to write `let`. | compile error |
| 1.5 | [functions-use-snake-case.kav](functions-use-snake-case.kav) | Function names must be snake_case. | compile error |
| 1.5 | [mixed-script-names-warn.kav](mixed-script-names-warn.kav) | A name that mixes letters from different scripts (a Latin `a` beside a Cyrillic `а`) gets a warning; a name written fully in one script gets none. | warning |
| 1.5 | [names-do-not-start-with-a-digit.kav](names-do-not-start-with-a-digit.kav) | A name is made of letters, digits and `_`, and must not start with a digit. | compile error |
| 1.5, 1.4 | [names-have-no-length-limit.kav](names-have-no-length-limit.kav) | A name has no length limit: 1,000-character names work for module constants, types, fields, functions, parameters, `const`, `let`, lambda parameters, patterns and `for` variables, in any script. | runs |
| 1.5, 1.6 | [names-may-begin-with-a-keyword.kav](names-may-begin-with-a-keyword.kav) | A name that starts with a keyword, or holds one, is an ordinary name for `const` and `let`. | runs |
| 1.5, 1.12 | [pattern-names-use-snake-case.kav](pattern-names-use-snake-case.kav) | A name a pattern takes is a variable, so it must be snake_case too. | compile error |
| 1.5 | [short-names-with-digits-and-underscores.kav](short-names-with-digits-and-underscores.kav) | One-letter names, names with digits after the first character, and names with `_` anywhere are names for `const` and `let`. | runs |
| 1.6 | [a-keyword-is-not-a-name.kav](a-keyword-is-not-a-name.kav) | A keyword can't be used as a name. | compile error |
| 1.6, 1.4 | [contextual-keywords-are-names-elsewhere.kav](contextual-keywords-are-names-elsewhere.kav) | Contextual keywords are ordinary names outside their place: `const` and `let` may use them and change them. | runs |
| 1.6 | [a-keyword-cant-name-a-function.kav](a-keyword-cant-name-a-function.kav) | A keyword used as a function's name is one error, at the name. | compile error |
| 1.6 | [a-keyword-cant-name-a-parameter.kav](a-keyword-cant-name-a-parameter.kav) | A keyword used as a parameter's name is one error, at the name. | compile error |
| 1.6 | [test-is-a-name-outside-a-test-block.kav](test-is-a-name-outside-a-test-block.kav) | `test` is a keyword only where a test block starts: elsewhere it names a function, a constant, a parameter or a field. | runs |
| 1.6, 11.8 | [a-test-block-beside-a-function-named-test.kav](a-test-block-beside-a-function-named-test.kav) | A test block still starts with `test` when a function is named `test` too. | tests pass |
| 1.8 | [long-expressions-compile.kav](long-expressions-compile.kav) | An expression has no length limit: a chain of 300 `+` terms compiles and runs. | runs |
| 1.12 | [a-name-with-a-default-is-never-none.kav](a-name-with-a-default-is-never-none.kav) | A pattern name with a default has the plain type `T`, so a `let` made that way can't be set to `none`. | compile error |
| 1.12 | [a-short-list-literal-leaves-names-optional.kav](a-short-list-literal-leaves-names-optional.kav) | A list literal with fewer items than the pattern has names leaves the names without an item `none`. | runs |
| 1.12, 1.4 | [destructured-const-field-cannot-change.kav](destructured-const-field-cannot-change.kav) | A record a `const` takes from a field is a constant: setting one of its fields is a compile error. | compile error |
| 1.12, 1.4 | [destructured-const-item-cannot-change.kav](destructured-const-item-cannot-change.kav) | A list a `const` takes from a field is a constant: setting an item of it, or of a list inside it, is a compile error. | compile error |
| 1.12, 1.4 | [destructured-const-names-cannot-be-reassigned.kav](destructured-const-names-cannot-be-reassigned.kav) | Names a `const` takes from a list are constants: assigning to one is a compile error. | compile error |
| 1.12, 1.4 | [destructured-const-names-cannot-use-compound-assignment.kav](destructured-const-names-cannot-use-compound-assignment.kav) | Names a `const` takes from a record are constants: `+=` on one is a compile error. | compile error |
| 1.12, 1.4 | [destructured-const-rest-cannot-push.kav](destructured-const-rest-cannot-push.kav) | A `...rest` list a `const` takes is a constant: calling `push` on it is a compile error. | compile error |
| 1.12, 6.1 | [destructured-let-names-can-change.kav](destructured-let-names-can-change.kav) | With `let`, every name a pattern takes is a variable: a name with a default, a `...rest`, a map name and a nested record can all change, and the value taken apart stays as it was. | runs |
| 1.12, 6.1, 3.3 | [destructured-names-are-copies.kav](destructured-names-are-copies.kav) | A name a `let` pattern takes is a value of its own: changing it later doesn't change a copy put in another variable, a list or a lambda before. | runs |
| 1.12 | [pattern-names-without-a-default-are-optional.kav](pattern-names-without-a-default-are-optional.kav) | A name a list pattern takes without a default is `T?`, so it can't be used as a plain `T`. | compile error |
| 1.12 | [rest-comes-last.kav](rest-comes-last.kav) | `...rest` may only be the last name of a list pattern. | compile error |
| 1.13, 1.4 | [a-local-may-hide-a-module-constant.kav](a-local-may-hide-a-module-constant.kav) | A function's body is an inner scope, so a local may reuse a module constant's name; other functions still read the constant. | runs |
| 1.13 | [a-module-constant-that-can-fail-needs-catch.kav](a-module-constant-that-can-fail-needs-catch.kav) | A module constant's value can't use anything that can fail without `catch`: it is worked out before `main`. | compile error |
| 1.13, 1.3 | [a-top-level-let-is-a-script-statement.kav](a-top-level-let-is-a-script-statement.kav) | In a script, a top-level `let`, and a `const` that names several values at once, are statements of `main`, run in their place; a plain `const` stays a module constant. | runs |
| 1.13 | [functions-read-no-top-level-let.kav](functions-read-no-top-level-let.kav) | A top-level `let` is a statement of `main`, not a module constant, so a function can't read it. | compile error |
| 1.13 | [functions-read-no-top-level-tuple-const.kav](functions-read-no-top-level-tuple-const.kav) | A top-level `const` that names several values at once is a statement of `main`, not a module constant, so a function can't read its names. | compile error |
| 1.13 | [module-constants-are-read-everywhere.kav](module-constants-are-read-everywhere.kav) | A module constant can be read by other constants, by lambdas (in a constant, in a function, in `main`), by a value a `catch` gives, and inside nested scopes. | runs |
| 1.13 | [module-constants-are-worked-out-in-order.kav](module-constants-are-worked-out-in-order.kav) | Module constants are worked out before `main`, in the order they are written, so a panic in the first one written comes first. | panic |
| 1.13, 1.4 | [module-constants-cannot-be-assigned.kav](module-constants-cannot-be-assigned.kav) | A module constant's value never changes: assigning to it in a function is a compile error. | compile error |
| 1.13, 1.4 | [module-constants-cannot-push.kav](module-constants-cannot-push.kav) | A module constant's value never changes: calling a changing method such as `push` on it is a compile error. | compile error |
| 1.13, 1.5 | [module-constants-use-snake-case.kav](module-constants-use-snake-case.kav) | A module constant's name is snake_case, as any value's: `maxCount` is an error. | compile error |
| 1.11 | [a-type-shows-as-its-display-gives.kav](a-type-shows-as-its-display-gives.kav) | A type implementing `Display` prints as its `display()` gives: with `print`, in interpolation, with `String(v)`, and inside collections and other records. | runs |
| 1.4 | [underscore-throws-a-value-away.kav](underscore-throws-a-value-away.kav) | `_` alone is not a name: it throws a value away, may be written any number of times, and reading it is a compile error. | compile error |
| 1.13 | [an-unread-module-constant-is-left-out.kav](an-unread-module-constant-is-left-out.kav) | A module constant nothing reads is left out: it isn't worked out before `main`, so its panic never happens. | warning |
| 1.14 | [tuple-assignment-sets-several-places.kav](tuple-assignment-sets-several-places.kav) | `(p, q) = (v, w)` sets several places at once: names, items, fields and dict entries. | runs |
| 1.14 | [tuple-assignment-sets-tuple-fields.kav](tuple-assignment-sets-tuple-fields.kav) | A tuple field of a `let` tuple is a place a tuple assignment can set. | runs |
| 1.14 | [tuple-assignment-takes-any-tuple-value.kav](tuple-assignment-takes-any-tuple-value.kav) | The right side may be any value whose type is a tuple, and tuples nest on both sides. | runs |
| 1.14 | [tuple-assignment-works-out-the-right-side-first.kav](tuple-assignment-works-out-the-right-side-first.kav) | Every value on the right is worked out before any place on the left is set. | runs |
| 1.14 | [tuple-assignment-sets-places-in-order.kav](tuple-assignment-sets-places-in-order.kav) | The places are set left to right, each worked out when it is set. | runs |
| 1.14 | [tuple-assignment-needs-one-value-per-place.kav](tuple-assignment-needs-one-value-per-place.kav) | The right side has one value for each place; another number is a compile error. | compile error |
| 1.14 | [tuple-assignment-values-must-fit.kav](tuple-assignment-values-must-fit.kav) | Each value must fit its place, as for `=`. | compile error |
| 1.14 | [tuple-assignment-places-must-change.kav](tuple-assignment-places-must-change.kav) | Each place is one `=` could set alone: a `const` among them is a compile error. | compile error |
| 1.14 | [tuple-assignment-is-only-equals.kav](tuple-assignment-is-only-equals.kav) | Only `=` sets several places at once: `(a, b) += (1, 1)` is a compile error. | compile error |
| 1.14 | [tuple-assignment-needs-parentheses.kav](tuple-assignment-needs-parentheses.kav) | Python's `a, b = b, a`, without parentheses, is a compile error that shows the form with them. | compile error |
| 1.10 | [print-joins-values-with-spaces.kav](print-joins-values-with-spaces.kav) | `print(a, b, …)` writes its values on one line, a space between them. | runs |
| 1.10 | [print-takes-sep-and-end.kav](print-takes-sep-and-end.kav) | `sep:` is the text between the values and `end:` the text after the last one, in either order, after the values. | runs |
| 1.10 | [print-with-no-values-writes-an-empty-line.kav](print-with-no-values-writes-an-empty-line.kav) | `print()` writes an empty line. | runs |
| 1.10 | [print-writes-text-among-values-as-it-is.kav](print-writes-text-among-values-as-it-is.kav) | Among several values, text is written as it is, while text inside a collection is quoted. | runs |
