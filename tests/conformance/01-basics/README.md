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
| 1.4 | [locals-only-after-let.kav](locals-only-after-let.kav) | A local variable can be used only after its `const`. | compile error |
| 1.4 | [no-duplicate-name-in-scope.kav](no-duplicate-name-in-scope.kav) | A name must not be declared twice in the same scope. | compile error |
| 1.4 | [unknown-name.kav](unknown-name.kav) | Using a name that was never declared is a compile error. | compile error |
| 1.4 | [var-can-change.kav](var-can-change.kav) | A `let` binding can be changed. | runs |
| 1.5 | [types-use-pascal-case.kav](types-use-pascal-case.kav) | Type names must be PascalCase. | compile error |
| 1.5 | [unicode-names.kav](unicode-names.kav) | Names may use letters from any Unicode script. | runs |
| 1.5 | [variables-use-snake-case.kav](variables-use-snake-case.kav) | Variable names must be snake_case. | compile error |
| 1.7 | [decimal-literal-from-context.kav](decimal-literal-from-context.kav) | A number literal takes its type from context, so a Decimal literal is exact. | runs |
| 1.7 | [integer-literals.kav](integer-literals.kav) | Integer literals may use `_` separators, hex (0x), octal (0o) and binary (0b). | runs |
| 1.7 | [multi-line-string.kav](multi-line-string.kav) | A multi-line string drops the line breaks next to its delimiters and the common indentation. | runs |
| 1.7 | [raw-string.kav](raw-string.kav) | A raw string has no escapes and no interpolation. | runs |
| 1.7 | [string-escapes.kav](string-escapes.kav) | Strings support the escapes \" \\ and \u{…}. | runs |
| 1.7 | [string-interpolation.kav](string-interpolation.kav) | Any expression inside {} in a string is inserted; \{ is a literal brace. | runs |
| 1.7 | [interpolation-closes-on-its-line.kav](interpolation-closes-on-its-line.kav) | In one-line text, a `{` must close on its line: `"{"` is one error at the `{`. | compile error |
| 1.7 | [interpolation-of-only-text-warns.kav](interpolation-of-only-text-warns.kav) | A `{…}` holding only text warns: `t == "{" or t == "}"` is a single string. | warning |
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
| 1.9 | [blocks-are-expressions.kav](blocks-are-expressions.kav) | A block has the value of its last expression. | runs |
| 1.11 | [values-print-like-literals.kav](values-print-like-literals.kav) | print shows values the way they are written in code. | runs |
| 1.12 | [destructuring-lists.kav](destructuring-lists.kav) | Lists come apart by position: missing items are `none` or their default; `...rest`; `_`. | runs |
| 1.12 | [destructuring-records.kav](destructuring-records.kav) | Records come apart by field name, renamed or with a default for a `T?` field. | runs |
| 1.12 | [destructuring-maps.kav](destructuring-maps.kav) | Maps come apart by key; a bare name is a text key. | runs |
| 1.12 | [destructuring-nests.kav](destructuring-nests.kav) | Patterns nest; inside a part that may be missing, names are `none` unless they have defaults. | runs |
| 1.12 | [destructuring-needs-the-field.kav](destructuring-needs-the-field.kav) | A record pattern names only fields the record has. | compile error |
| 1.12, 8.7 | [destructuring-keeps-secrets.kav](destructuring-keeps-secrets.kav) | A name taken from a `secret` field is a secret. | compile error |
| 1.12 | [list-patterns-are-not-for-match.kav](list-patterns-are-not-for-match.kav) | List and record patterns go in `const` and `let`, not `match`. | compile error |
