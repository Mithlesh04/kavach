# 1. Basics

## 1.1 Source files

- Source files are UTF-8 text with the extension `.kav`. A byte order mark (U+FEFF), which some Windows editors put at a file's start, counts as a space.
- A statement ends at the end of a line. There are no semicolons.
- A statement continues on the next line when the line ends with an operator, `,`, `(`, `[` or `{`, or when the next line starts with `.` (method chains).
- Inside `( … )` and `[ … ]`, line breaks never end a statement. Inside `{ … }` they do, even when the braces are inside parentheses (a lambda body in a call).
- `<`, `>`, `>>` and `?` don't continue a line, because they also close types: `const names: Set<String>` and `const age: Int?` end where the line ends.

```kavach
const total = orders
    .filter(.status == Status.Paid)
    .sum(.amount)
```

## 1.2 Comments

```kavach
// A line comment.

/* A block comment, which may run
   over several lines. */

/// A doc comment. It describes the declaration below it.
/// Editors show it on hover, and `ask<T>` uses it to describe fields to the AI model.
fn area(width: Float, height: Float) -> Float { width * height }
```

- A block comment may hold others: `/* old code /* with a comment */ */` is one comment, so code that has comments can be put aside whole.
- It may sit inside a line, between any two tokens: `print(a + /* the tax */ 2)`. A line break inside it doesn't end the statement.
- A `/*` that is never closed with `*/` is an error.
- Doc comments are only `///`.

## 1.3 Entry point

A program starts at `fn main()`.

```kavach
fn main() {
    print("Hello, Kavach!")
}
```

For scripts, the entry file may contain top-level statements instead. They become the body of `main`.

```kavach
print("Hello, Kavach!")
```

Only the entry file may contain top-level statements.

## 1.4 Variables

```kavach
const name = "Asha"    // a constant: never changes
let count = 0          // a variable: can change
count += 1

const limit: Int = 10  // an explicit type is optional
```

- `const` names a value that never changes; `let` names a variable ([decision 0034](../decisions/0034-const-and-let.md)). Use `const` unless the value must change.
- Assigning to a `const`, or calling a method that changes it (`push`), is a compile error that says to use `let`.
- `var` is not Kavach: writing it is a compile error that says to write `let`.
- A name must not be declared twice in the same scope. Inner scopes may reuse a name.
- Every name must be declared. Using an unknown name is a compile error, and the error suggests similar names (`did you mean total?`).
- Declarations at the top of a module (`fn`, `type`, `interface`, …) can be used anywhere in that module, before or after they appear. Local variables can be used only after their `const` or `let`.
- A variable that is never used (a `const`, a `let`, a name a pattern takes, `...rest`, a `for` variable, a `catch` name) gets a warning, so code being written still runs. A name starting with `_` (`_draft`, or `_` alone) is unused on purpose and gets none.
- A never-used variable that hides a `let` of the same name in an outer scope says so, since it is almost always meant to change that one: `let total = total + i` inside a loop makes a new `total`. The warning says to write `total = total + i`.
- Native code doesn't compute a value nobody reads, when computing it does nothing a program could see: an unused `...rest` makes no list.

## 1.5 Names

- Made of letters (any Unicode script), digits and `_`. Must not start with a digit.
- `snake_case` for variables, functions and modules.
- `PascalCase` for types, union variants, interfaces, capabilities and agents.

The compiler enforces these styles, and `kavach fmt` fixes them automatically: it renames the declaration and every use. Two names it leaves alone, and says so: a `pub` name, which other files may use, and one whose fixed spelling is already taken. One style across all Kavach code makes every codebase look familiar.

A name that mixes letters from different scripts (for example a Latin `a` and a Cyrillic `а` in one name) is a **warning**: look-alike letters can hide malicious code. Names written fully in one script, such as `नाम`, are fine.

## 1.6 Keywords

```
agent       and         arena       as          assert      break
capability  catch       const       continue    deprecated  else
ensure      example     extend      extern      fail        false
fn          for         if          in          interface   invariant
let         match       none        not         or          pub
require     return      secret      self        spawn       test
true        try         type        unsafe      use         uses
var         while       with
```

`var` is reserved so that writing it gives a clear error (1.4); it declares nothing.

**Contextual keywords** are keywords only in one place, and ordinary names everywhere else:

| Word | Keyword only in |
|---|---|
| `result` | `ensure` and `example` clauses: the return value (7.2) |
| `fails` | `example` clauses: the expected error (7.6) |
| `fix` | `deprecated` declarations: the replacement call (12.6) |
| `context` | After a `try` expression: text added to a passed-on error (5.7) |
| `can`, `limit`, `approve`, `model`, `on` | At the start of a line inside an `agent` body (8.3) |
| `when`, `by` | Inside an `approve` rule (8.3) |
| `per` | Inside a `limit` rule: the time window (8.3) |
| `inout` | Before a parameter's name: the function changes the caller's variable (3.9) |

## 1.7 Literals

| Kind | Examples |
|---|---|
| Integer | `42`, `1_000_000`, `0xff` (hex), `0o644` (octal), `0b1010` (binary) |
| Float | `3.14`, `1e9`, `2.5e-3` |
| Bool | `true`, `false` |
| Char | `'a'`, `'क'`, `'\n'` |
| String | `"Hello, {name}!"` |
| Multi-line string | `"""` … `"""` (common leading indentation is removed) |
| Raw string | `r"C:\path\{not interpolated}"` |
| Empty optional | `none` |

In a multi-line string, the line break right after the opening `"""` and the one right before the closing `"""` are not part of the string. Then the common leading indentation is removed:

```kavach
const text = """
    first
      second
    """
// text is "first\n  second"
```

Number literals take their type from context. `const price: Decimal = 499.99` is an exact decimal, not a float. Without context, integers are `Int` and decimals are `Float`.

A minus written before a whole-number literal belongs to it, so the smallest value of each type can be written: `const low: Int8 = -128`, though `128` alone doesn't fit `Int8`, and `-9223372036854775808` is the smallest `Int`.

### String interpolation

Any expression inside `{}` in a string is inserted into it. Write `\{` for a literal brace.

```kavach
print("Order {order.id} costs {order.amount}")
print("Use \{ and } in templates")
```

Escapes: `\n`, `\t`, `\\`, `\"`, `\{`, `\u{1F600}`.

In one-line text, a `{` must close on the same line: `"{"` is a compile error at the `{`. A `{…}` that holds only text gives a warning, because it is almost always a brace that needed `\{`: `t == "{" or t == "}"` is a single string, the text ` or t == ` inside `{…}`, not two comparisons.

## 1.8 Operators

| Group | Operators | Notes |
|---|---|---|
| Arithmetic | `+ - * / % **` | `Int / Int` is an `Int` (truncated). Overflow is checked (see 5.6). `a ** b` is `a` to the power `b`, both the same number type; for whole numbers it panics when `b` is negative or the result doesn't fit |
| Comparison | `== != < <= > >=` | |
| Logic | `and or not` | Words, not symbols, for readability: `!ready`, `&&` and `\|\|` are compile errors that say which word to write |
| Bits | `& \| ^ << >> ~` | Integers only. `a << n` panics when the result doesn't fit (5.6); both shifts panic when `n` is negative or not less than the type's width in bits. `>>` keeps the sign |
| Optional | `??` `?.` | See 2.7 |
| Range | `..` `..=` | `0..10` excludes 10; `0..=10` includes it |
| Assignment | `= += -= *= /= %= **= &= \|= ^= <<= >>=` | Assignment is a statement, not an expression. `x op= v` is `x = x op v`, checked as that operator is |

Precedence, from highest to lowest:

1. `.` `?.` calls, indexing
2. `**`, right to left: `2 ** 3 ** 2` is `2 ** 9`. It binds tighter than a minus on its left, as in mathematics: `-2 ** 2` is `-(2 ** 2)`, `-4`
3. `not` `~` unary `-`
4. `* / %`
5. `+ -`
6. `<< >>`
7. `&`, then `^`, then `|`
8. `.. ..=`
9. `== != < <= > >=`
10. `and`
11. `or`
12. `??`

```kavach
let flags = 0b1010
flags &= 0b0110                        // 0b0010
flags <<= 2                            // 0b1000
print(2 ** 10)                         // 1024
print(2.0 ** 0.5)                      // 1.4142135623730951
print(-2 ** 2)                         // -4
```

When in doubt, use parentheses. `kavach fmt` never removes parentheses that help readability.

**Equality** (`==`, `!=`) compares contents, at every depth: two lists are equal when they hold equal items in the same order, two records when their fields are equal, two maps and two sets when they hold the same keys (with equal values), **in any order**. Shared values are equal at once: a copy not yet changed (spec 6.1) is not looked inside ([decision 0039](../decisions/0039-equality-is-deep-and-fast.md)).

```kavach
const a = ["x": [1, 2], "y": [3]]
const b = a
print(a == b)                          // true, in one step: b is a's copy
print(["a": 1, "b": 2] == ["b": 2, "a": 1])   // true: order doesn't count
print([1, 2] == [2, 1])                // false: lists keep order
```

**Order** (`<`, `<=`, `>`, `>=`, and `sort`, `sort_by`, `min`, `max` in 14.2) is defined for numbers, text, characters, `Bool` and tuples of them. Other values have no order, and comparing them is a compile error.

- Text compares character by character, by Unicode code point: every capital letter comes before every small one, and letters with accents come after both, so `"Apple" < "apple" < "zebra" < "Éclair"`. It is not dictionary order for any language.
- `false` comes before `true`.
- Tuples compare item by item: the first pair that differs decides, so sorting by `(age, name)` sorts by age, then by name among equal ages.

```kavach
print((30, "Asha") < (30, "Ravi"))     // true: equal ages, so the names decide
const by_age = people.sort_by(p => (p.age, p.name))
```

**Comparisons chain**, as in mathematics and Python: `a op1 b op2 c` means `a op1 b and b op2 c`, for any mix of `== != < <= > >=` and any length ([decision 0040](../decisions/0040-chained-comparisons.md)). Each operand is worked out once, left to right, and the chain stops at the first link that is false. Only neighbours are compared: `a != b != c` says nothing about `a` and `c`. Parentheses group as usual, and ranges don't chain.

```kavach
if 0 <= i < items.len() { print(items[i]) }
print(b == a == c)                     // b == a and a == c
print(1 < f() <= 10)                   // f is called once
```

**Three things that work but rarely mean what they say** give a warning:

- `==` or `!=` on floats worked out while the program runs. Rounding makes most such values differ in their last bits, so `a + 0.2 == 0.3` is `false` for `a = 0.1`. Use `a.near(b)` (14.2), or `Decimal` for exact amounts. Comparing with a whole number (`x == 0.0`), comparing two constants (2.1), and `example` lines give no warning.
- `x % 2 == 1` on a signed number. `%` keeps the sign of `x`, so `-3 % 2` is `-1` and the test misses negative odd numbers. Write `x % 2 != 0`.
- `/` between two whole-number literals that don't divide evenly. `1 / 2` is `0`, since whole numbers divide to a whole number. Write `1.0 / 2.0` for `0.5`, or `1.div_floor(2)` when the whole part is meant.

**Arithmetic that always panics** is a compile error, when the compiler knows the values: a whole-number division or remainder by `0`, `+`, `-` or `*` of known values whose result doesn't fit the type, an index outside a list whose length is known (a list literal, or a `const` set to one), and `.unwrap()` on a value known to be `none` (2.7). The message is the one the run would give. A known value is a literal (`none` too), a `const` set to one, a known whole number converted to a whole-number type it fits (`Int8(i)`), or a `let` set to one with nothing in between that may change it: a branch, a loop or a lambda may or may not run, so a `let` it sets is known no more, nor is a `let` an `if`, `while` or `match` tests, since the test may be what keeps the arithmetic safe.

A `for` over a range of known numbers counts too, when nothing in its body can leave early (`break`, `continue`, `return`, `fail` or `try`): each statement that always runs in the body is checked for the loop's first and last values, and a `+=` or `-=` of a known number for all its rounds. The message is that of the first round that panics. A body that can leave early may be what keeps that round from running, so it gets no error; nor does a statement inside an `if`, a `match` or an inner loop that may not run.

```kavach
const limit: Int32 = 2147483647
print(limit + 1)      // error: this always panics: integer overflow: 2147483647 + 1 doesn't fit in Int32
print(total / 0)      // error: this always panics: division by zero
const sizes = [1, 2, 3]
print(sizes[5])       // error: this always panics: index 5 is out of range: the list has 3 items

let found: Int? = none
print(found.unwrap()) // error: this always panics: `.unwrap()` on `none`

let count: Int8 = 120
count += 7
count += 1            // error: this always panics: integer overflow: 127 + 1 doesn't fit in Int8

let safe: Int8 = 127
if safe < 127 {
    safe += 1         // no error: the `if` tests it
}

for i in 0..4 {
    print(sizes[i])   // error: this always panics: index 3 is out of range: the list has 3 items
}
for d in 0..3 {
    print(12 / d)     // error: this always panics: division by zero
}
let hits: Int8 = 0
for _ in 0..200 {
    hits += 1         // error: this always panics: integer overflow: 127 + 1 doesn't fit in Int8
}
for i in 0..100 {
    print(Int8(i) * 2)  // error: this always panics: integer overflow: 64 * 2 doesn't fit in Int8
}
```

## 1.9 Blocks are expressions

A block `{ … }` has the value of its last expression.

```kavach
const grade = if score >= 90 { "A" } else { "B" }
```

## 1.10 Printing

- `print(value)` writes a line to standard output. It needs the `Console` capability, which `main` has (chapter 8).
- `debug(value)` prints the value with its file and line, to standard error. It works anywhere, needs no capability, and is **removed from release builds**: `kavach run` prints it, native code (`kavach build`, `kavach run --native`) leaves it out, argument and all. So `debug(reveal(secret))` shows a secret while you develop and never in production.

## 1.11 How values are printed

`print` and `{}` in strings show a value the way you would write it in code, so output is never ambiguous:

| Value | Printed as |
|---|---|
| `Int` | `42`, `-7` |
| `Float` | Shortest form that reads back exactly, always with a `.` or an exponent: `7.0`, `0.1`, `1e21` |
| `Decimal` | All its digits, keeping its scale: `499.00` prints `499.00` |
| `Bool`, `Char` | `true`, `क` |
| `String` | The text itself; inside a collection or record it is quoted: `"Asha"` |
| Optional | The value, or `none` |
| List, map, tuple | `[1, 2]`, `["asha": 31]`, `(200, "OK")` |
| Record | `User(name: "Asha", age: 31)` |
| Union variant | `Paid`, `TooLarge(value: 200)` |
| Secret field | `<secret>` |

A type can choose its own form by implementing the `Display` interface (`fn display(self) -> String`).

## 1.12 Destructuring

`const` and `let` can take a value apart, as JavaScript does, with Kavach's types ([decision 0037](../decisions/0037-destructuring-as-javascript-does.md)). Tuples do it by position (2.3); lists by position; records by field name.

```kavach
const [first, second] = scores            // first: Int?, second: Int?
const [low = 0, high = 100] = limits      // low: Int, high: Int
const [head, ...rest] = names             // head: String?, rest: [String]
const [_, second_name] = names            // `_` skips an item
const [two, three] = [2, 3]               // two: Int, three: Int

const { name, age: years } = user         // name: String, years: Int
const { city = "unknown" } = address      // city is `String?` in the record: String here
```

- **A list** may be shorter than the pattern: a name with no item gets `none`, so its type is `T?`. With a default (`= value`), the name gets the default instead, and its type is `T`. Items after the pattern are ignored.
- `...rest` (last only) gets the items after the others, as a list: empty when there are none.
- When the value is a list literal with an item for every name (`[2, 3]`), no name can be missing, so the names have the items' own types.
- **A record** is taken apart by field name; `age: years` names the variable differently. A field the record doesn't have is a compile error. A default fills a field of type `T?` when it is `none`, giving a `T`.
- **A map** is taken apart by key, its pattern written like a map literal: `"asha": a` reads key `"asha"` into `a`; a bare name is the key spelled like it (`port` reads `"port"`), as in JavaScript's `{ port }`. A key may be missing, so a name is `V?`, or `V` with a default. A bare name or `name: …` is a text key; other keys are written as literals (`7: seven`).

```kavach
const ["asha": a, "ravi": r] = ages       // a: Int?, r: Int?
const [host: server, port = 8080] = settings   // server: String?, port: Int
```

- Patterns nest: an item, a map value or a field can be taken apart in turn, by a list, map or record pattern in its place.

```kavach
const [first, [x, y]] = pairs             // pairs: [[Int]]; x and y are Int?
const [_, [low = 0, high = 0]] = pairs    // low and high are Int
const { home: { city } } = user           // home: Address, so city: String
const [config: [port = 8080]] = settings  // settings: [String: [String: Int]]
```

- When the part a nested pattern takes apart may be missing (a list item, a map value, a `T?` field), every name inside it is `none` when it is, so their types are optional, unless they have defaults. A part with a default of its own (`[x, y] = [0, 0]`) is never missing.
- With `let`, the names are variables; with `const`, they never change.

## Open questions

- **List patterns in `match`:** there a pattern would test the length (`[a, b]` only for two items), unlike `const`, which never fails. Wanted, but the two meanings must not be confused.

Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
