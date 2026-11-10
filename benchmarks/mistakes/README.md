# benchmarks/mistakes/

Common mistakes from real code, each written as a small program, and the step that catches it **before the program runs** (to-do item 5). A mistake caught while the code is typed costs seconds; the same mistake found in production costs much more.

The mistakes come from [classic-mistakes.md](../../docs/vision/classic-mistakes.md) (the traps other languages are known for) and from what people write under deadline pressure: misspellings, wrong types, a missing case, an index one past the end.

## The steps

| Step | What it means |
|---|---|
| compile error | `kavach check`, `kavach run` and the editor refuse the program and say why |
| warning | the program runs, and the editor and `kavach check` show the warning while it is typed |
| …, auto-fix | an error or a warning with one right fix: the editor's quick fix, or `kavach fix` for whole folders (spec 12.7) |
| only when it runs | nothing finds it before a run: a panic, or a wrong answer |

## Results

Measured 2026-11-10 with `kavach check` and `kavach fix`, at commit time. 28 of 28 caught before a run, 7 of them with an auto-fix.

| Mistake | Caught by | Message |
|---|---|---|
| [`=` written where `==` was meant](cases/assignment-as-condition.kav) | compile error, auto-fix | `=` sets a variable: to compare, write `==` (spec 4.1) |
| [A call that breaks the function's own contract](cases/broken-contract.kav) | compile error | this call always breaks `require x > 0` of `half`: `x` is -4 (spec 7.7) |
| [A return that breaks the function's own promise](cases/broken-ensure.kav) | compile error | this always breaks `ensure result >= 0` of `clamp_low`: `result` is -1 (spec 7.7) |
| [A list declared constant, then changed](cases/changing-a-constant.kav) | compile error, auto-fix | `push` changes `xs`, but it is a `const`: declare it with `let` |
| [A count that overflows after it was changed](cases/changing-let-overflows.kav) | compile error | this always panics: integer overflow: 127 + 1 doesn't fit in Int8 (spec 1.8) |
| [A division by zero](cases/divide-by-zero.kav) | compile error | this always panics: division by zero. Divide by something that can't be 0 (spec 1.8) |
| [An `else if` an earlier condition already covers](cases/else-if-never-true.kav) | warning, auto-fix | this condition is never true here: `score > 50` means `score > 10`, and that branch is taken above. Check the order of the conditions (spec 4.7) |
| [The largest item of a list that may be empty](cases/empty-list-max.kav) | compile error | expected a number, found `Int?`: it may be `none`, so give a default with `?? 0`, or check it with `if let` (spec 2.7) |
| [A `while` whose variable never changes](cases/endless-while.kav) | warning | this loop never ends: `n` and `limit` don't change in it, and nothing in it leaves it. Change them in the loop, add a `break`, or write `while true` if it should run forever (spec 4.7) |
| [Floats compared with `==` after arithmetic](cases/float-equality.kav) | warning | `==` on floats compares every bit, and rounding makes most computed values differ (`0.1 + 0.2` worked out while running isn't `0.3`): use `a.near(b)`, or `Decimal` for exact amounts |
| [A public function that reads a file without saying so](cases/hidden-file-access.kav) | compile error, auto-fix | `price` is `pub`, so it must say what it uses: add `uses Fs.read` (spec 8.1) |
| [A literal index past the end of a literal list](cases/index-past-end-known.kav) | compile error | this always panics: index 5 is out of range: the list has 3 items (spec 1.8) |
| [A loop whose range runs past the end of a list](cases/index-past-the-end-in-a-loop.kav) | compile error | this always panics: index 3 is out of range: the list has 3 items (spec 1.8) |
| [Whole-number division where a fraction was meant](cases/integer-division-surprise.kav) | warning | `1 / 2` is `0`: whole numbers divide to a whole number. Write `1.0 / 2.0` for a fraction, or `1.div_floor(2)` when the whole part is meant (spec 1.8) |
| [A `match` that forgets a case](cases/missing-case.kav) | compile error | this `match` does not cover `Status.Refunded` |
| [A name misspelled](cases/misspelled-name.kav) | compile error | `totl` is not defined; did you mean `total`? |
| [`x % 2 == 1` used as the odd test](cases/odd-test-on-negatives.kav) | warning, auto-fix | `x % 2 == 1` is wrong for a negative odd `x`, since `-3 % 2` is `-1`: write `x % 2 != 0` |
| [A loop one past the end of a list](cases/off-by-one.kav) | warning, auto-fix | `i` reaches `xs.len()`, one past the last index of `xs`, so `xs[i]` panics on the last turn: write `..` instead of `..=`, or loop over `xs` itself (spec 4.4) |
| [A value that may be missing, used as if it were there](cases/optional-used-as-value.kav) | compile error | expected a number, found `Int?`: it may be `none`, so give a default with `?? 0`, or check it with `if let` (spec 2.7) |
| [An overflow the values already show](cases/overflow-known.kav) | compile error | this always panics: integer overflow: 2147483647 + 1 doesn't fit in Int32 (spec 1.8) |
| [A secret printed to the log](cases/secret-printed.kav) | compile error | `number` is secret, so it can't go into `print`; use `reveal(…)` if you really mean to |
| [A running total that shadows itself in a loop](cases/shadowed-total.kav) | warning | `total` is never used: this makes a new `total`, hiding the one declared above. To change that one, write `total = …` without `let` (spec 1.4) |
| [Text added to a number](cases/text-plus-number.kav) | compile error | expected `String`, found `Int` |
| [An error left unhandled](cases/unhandled-error.kav) | compile error | `read_text` can fail: handle the error with `try`, `catch` or `match` |
| [A `match` arm that can never match](cases/unreachable-arm.kav) | warning, auto-fix | this arm never matches: the arm above it matches every value first. Remove it, or move it up (spec 4.7) |
| [A result computed and thrown away](cases/unused-result.kav) | compile error | the result of `total` is unused: use it, or write `const _ = …` to throw it away |
| [`.unwrap()` on a value still `none`, set before a step that was meant to fill it](cases/unwrap-of-none.kav) | compile error | this always panics: `.unwrap()` on `none`. Handle `none` with `if let`, `match` or `?? default` (spec 1.8) |
| [Text passed where a number is wanted](cases/wrong-argument-type.kav) | compile error | expected `Int`, found `String` |

## Running it

```
cargo build -p kavach-cli
python benchmarks/mistakes/run.py [path to kavach]
```

It checks every file in [cases/](cases/), gives a copy of each to `kavach fix` (in a folder it removes after), and prints the table above. To measure a new mistake, add `cases/<name>.kav` whose first line is `// Mistake: <what it is>.`
