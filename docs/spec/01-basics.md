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
    print("Hello, world!")
}
```

`main` takes no parameters: `fn main(args: [String])` is a compile error. A program reads its arguments with `cli.args()` (14.6).

For scripts, the entry file may contain top-level statements instead. They become the body of `main`. A top-level `const` is a module constant instead (1.13), in any file, even one with `fn main`; in a script, one that uses a capability stays a statement, as 1.13 shows.

```kavach
print("Hello, world!")
```

Only the entry file may contain top-level statements, and only when it has no `fn main`: there they would never run, so a statement outside `main` in a file with `fn main` is a compile error. Move it into `main`, or make it a `const`.

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
- Nor is `let mut`: a `let` can already change. `let mut count = 0` is a compile error that says to write `let count`.
- A name must not be declared twice in the same scope. Inner scopes may reuse a name.
- So two functions can't share a name: Kavach has no overloading. Where other languages overload, a function takes a default argument (3.2), a union (2.6) or a generic type (2.9), or the two get different names. The error says so, and calls go to the first function.
- Every name must be declared. Using an unknown name is a compile error, and the error suggests similar names (`did you mean total?`).
- Declarations at the top of a module (`fn`, `type`, `interface`, …) can be used anywhere in that module, before or after they appear. Local variables can be used only after their `const` or `let`.
- A variable that is never used (a `const`, a `let`, a name a pattern takes, `...rest`, a `for` variable, a `catch` name) gets a warning, so code being written still runs. A name starting with `_` (`_draft`, or `_` alone) is unused on purpose and gets none. Functions, types and module constants nothing uses get one too (10.1). `_` alone is not a name: it throws the value away, so it may be written any number of times and is never read; reading it (`print(_)`) is a compile error that says to give the value a name, such as `_t`.
- A never-used variable that hides a `let` of the same name in an outer scope says so, since it is almost always meant to change that one: `let total = total + i` inside a loop makes a new `total`. The warning says to write `total = total + i`.
- A function or method parameter that the function never reads gets a warning too: "`times` is never used: remove it, or name it `_times` if it must stay". A read inside a lambda in the body counts, and so does a read in a `require` or `ensure` (7.1, 7.2): the contract is checked at every call. A parameter named `_` or starting with `_` gets none. Nor does one whose signature something else fixes: a method that implements an interface or a capability (2.8, the operators of 2.16, 8.2), a function used as a value anywhere in the program (passed, stored, or named by a `literal`'s `check` or `param`; 3.4, 10.15), `main`, an agent's `on` handler (8.3), a `gpu fn` (15) and a function with no body (`extern`, 10.7). A lambda's parameters get no warning.

  ```kavach
  fn repeat(text: String, times: Int) -> String {   // warning: `times` is never used
      text
  }

  fn shout(text: String, _times: Int) -> String {   // `_times`: kept on purpose
      text.upper()
  }

  fn twice(text: String, times: Int) -> String     // read only by the contract: used
      require times == 2
  {
      text + text
  }
  ```
- Native code doesn't compute a value nobody reads, when computing it does nothing a program could see: an unused `...rest` makes no list.

## 1.5 Names

- Made of letters (any Unicode script), digits and `_`. Must not start with a digit.
- `snake_case` for variables, functions and modules.
- `PascalCase` for types, union variants, interfaces, capabilities and agents.

The compiler enforces these styles: a name in the wrong style is an error that carries its fix, which `kavach fix` and the editor's quick fix apply, renaming the declaration and every use. `kavach fmt` sets only the layout and never renames (12.7; a maintainer, 2026-11-27; *not built yet*: today `kavach fmt` renames too). Two names it leaves alone, and says so: a `pub` name, which other files may use, and one whose fixed spelling is already taken. One style across all Kavach code makes every codebase look familiar.

A name that mixes letters from different scripts (for example a Latin `a` and a Cyrillic `а` in one name) is a **warning**: look-alike letters can hide malicious code. Names written fully in one script, such as `नाम`, are fine.

## 1.6 Keywords

```
agent       and         arena       as          assert      break
capability  catch       const       continue    deprecated  else
ensure      example     extend      extern      fail        false
fn          for         if          in          interface   invariant
let         match       none        not         or          pub
require     return      secret      self        spawn       true
try         type        unsafe      use         uses        var
while       with
```

`var` is reserved so that writing it gives a clear error (1.4); it declares nothing. A keyword used as a name is one error that says so and suggests another name: `` `while` is a keyword, so it can't name a function: choose another name ``. The compiler reads on past it, so the rest of the file is still checked:

```kavach
fn while(n: Int) -> Int {    // error: `while` is a keyword, so it can't name a function
    n + 1
}
```

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
| `is` | Between a value and a type: `x is Circle`, a type check that narrows `x` (2.15; *not built yet*) |
| `test` | At the start of a line at a file's top level, followed by text in quotes: a test block, `test "adds two numbers" { … }` (11.8). Everywhere else it is an ordinary name: `fn test()`, `const test = …` (a maintainer, 2026-11-27) |

## 1.7 Literals

| Kind | Examples |
|---|---|
| Integer | `42`, `1_000_000`, `0xff` (hex), `0o644` (octal), `0b1010` (binary) |
| Float | `3.14`, `1e9`, `2.5e-3` |
| Bool | `true`, `false` |
| Char | `'a'`, `'क'`, `'\n'` |
| String | `"C:\temp\{name}"`: exactly what is written, on one line |
| Template | `` `Hello, {name}!` ``: fills in `{…}` and reads escapes; the one form for text over lines |
| Empty optional | `none` |

Number literals take their type from context. `const price: Decimal = 499.99` is an exact decimal, not a float. Without context, integers are `Int` and decimals are `Float`.

A minus written before a whole-number literal belongs to it, so the smallest value of each type can be written: `const low: Int8 = -128`, though `128` alone doesn't fit `Int8`, and `-9223372036854775808` is the smallest `Int`.

### Plain strings and templates

Text in quotes is exactly what is written, on one line: no escapes, and nothing filled in ([decision 0131](../decisions/0131-plain-strings-and-backtick-templates.md)). Paths and regular expressions need nothing special. Text that holds a `"`, as JSON almost always does, or goes over lines, is written in backticks, with `\{` for each brace (`` `{"id": 7}` `` would fill in a value).

```kavach
const path = "C:\temp\{name}"          // these characters, backslashes and braces too
const pattern = "\d+\.\d+"
const digits = "\d{3}"                 // the 5 characters as written
```

Text in backticks is a **template**: any expression inside `{}` is filled in, and escapes are read.

```kavach
print(`Order {order.id} costs {order.amount}`)
print(`first\nsecond\ttab \u{1F600}`)
print(`a literal \{ brace and a \` backtick`)
```

- Escapes in a template: `\n`, `\r` (a carriage return, as protocols that end lines with `\r\n` need), `\t`, `\\`, `` \` ``, `\{`, `\u{1F600}`. A `Char` reads the same escapes: `'\n'`.
- In a template, a `{` must close on its own line.
- A string in quotes that looks like it wanted a template gets a warning that says to write it in backticks: `"Hello, {name}"` when `name` is a name in scope there, and `"a\nb"` when every backslash is `\n`, `\t` or `\r` and the text isn't a Windows path; `"…"` holding braces or backslashes meant as text needs nothing.
- `r"…"` and Python's `f"…"` are compile errors that say what to write: a quoted string is raw already, and a template is written in backticks.

### Text over lines

Text over lines is written only in backticks ([decision 0143](../decisions/0143-text-over-lines-in-backticks.md)). A template whose text starts on the line after its opening backtick spans lines:

- The line break right after the opening backtick is left out. So is the one right before the closing backtick, when that backtick is on a line of its own.
- Then the common leading indentation is removed: the smallest indentation of the lines that hold text. A line indented deeper keeps the rest of its indentation.
- Empty lines stay. A line of only spaces doesn't count when the common indentation is found, and loses as much of it as it has.
- It is a template like any other: `{…}` is filled in and escapes are read, so `\{` writes a brace, `` \` `` a backtick and `\\` a backslash. `"` and `'` need no escape.
- A template whose text starts on the same line as its opening backtick stays on that line: a line break in its text is a compile error that says to start the text on the line after the backtick.
- `"""…"""` is a compile error that shows the backtick form: write the text in backticks, with `\{` for a brace, `` \` `` for a backtick and `\\` for a backslash. So is a tagged `sql"""…"""` (10.15), which is written `` sql`…` ``.

```kavach
const text = `
    first
      second
    `
// text is the two lines "first" and "  second"

const body = `
    \{"name": "{name}", "tags": ["a", "b"]}
    `
// with name "Ana", body is {"name": "Ana", "tags": ["a", "b"]}
```

## 1.8 Operators

| Group | Operators | Notes |
|---|---|---|
| Arithmetic | `+ - * / % ** @` | `Int / Int` is an `Int` (truncated). Overflow is checked (see 5.6). `a ** b` is `a` to the power `b`, both the same number type; for whole numbers it panics when `b` is negative or the result doesn't fit. `+` also joins two `String`s, two `Bytes` (2.13), and two lists of the same type (2.2). A program's own types get operators through interfaces (2.16). `a @ b` is the matrix product (14.33), for a type with `MatMul` |
| Comparison | `== != < <= > >=` | |
| Logic | `and or not` | Words, not symbols, for readability: `!ready`, `&&` and `\|\|` are compile errors that say which word to write |
| Bits | `& \| ^ << >> ~` | Integers only. `a << n` panics when the result doesn't fit (5.6); both shifts panic when `n` is negative or not less than the type's width in bits. `>>` keeps the sign |
| Optional | `??` `?.` | See 2.7 |
| Range | `..` `..=` | `0..10` excludes 10; `0..=10` includes it |
| Index and slice | `x[i]` `x[a:b:step]` `x[#k]` `m[i, j]` | One item, a slice, `k` from the end, one part per dimension (2.19). A program's own types get them through interfaces (2.16) |
| Assignment | `= += -= *= /= %= **= @= &= \|= ^= <<= >>=` | Assignment is a statement, not an expression. `x op= v` is `x = x op v`, checked as that operator is. `(a, b) = (b, a)` sets several places at once (1.14) |

Precedence, from highest to lowest:

1. `.` `?.` calls, indexing and slicing
2. `**`, right to left: `2 ** 3 ** 2` is `2 ** 9`. It binds tighter than a minus on its left, as in mathematics: `-2 ** 2` is `-(2 ** 2)`, `-4`
3. `not` `~` unary `-`
4. `* / % @`
5. `+ -`
6. `<< >>`
7. `&`, then `^`, then `|`
8. `.. ..=`
9. `== != < <= > >=`
10. `and`
11. `or`
12. `??`, right to left: `a ?? b ?? c` is `a ?? (b ?? c)`, so `b` may be an optional too, and the first value present wins

```kavach
let flags = 0b1010
flags &= 0b0110                        // 0b0010
flags <<= 2                            // 0b1000
print(2 ** 10)                         // 1024
print(2.0 ** 0.5)                      // 1.4142135623730951
print(-2 ** 2)                         // -4
```

When in doubt, use parentheses. `kavach fmt` never removes parentheses that help readability.

Operands are worked out left to right, as written: the left one before the right one, and a method's value before its arguments ([3.10](03-functions.md#310-the-order-things-are-worked-out-in)).

**Equality** (`==`, `!=`) compares contents, at every depth: two lists are equal when they hold equal items in the same order, two records when their fields are equal, two dicts and two sets when they hold the same keys (with equal values), **in any order**. Shared values are equal at once: a copy not yet changed (spec 6.1) is not looked inside ([decision 0039](../decisions/0039-equality-is-deep-and-fast.md)).

```kavach
const a = {"x": [1, 2], "y": [3]}
const b = a
print(a == b)                          // true, in one step: b is a's copy
print({"a": 1, "b": 2} == {"b": 2, "a": 1})   // true: order doesn't count
print([1, 2] == [2, 1])                // false: lists keep order
```

**Order** (`<`, `<=`, `>`, `>=`, and `sort`, `sort_by`, `min`, `max` in 14.2) is defined for numbers, text, characters, `Bool`, and tuples, lists, rings and text views (2.18) of them, and `Bytes`. Other values have no order, and comparing them is a compile error: a set or a dict has none (compare `s.to_list().sort()` instead), nor an optional.

- Text compares character by character, by Unicode code point: every capital letter comes before every small one, and letters with accents come after both, so `"Apple" < "apple" < "zebra" < "Éclair"`. It is not dictionary order for any language.
- `false` comes before `true`.
- Tuples compare item by item: the first pair that differs decides, so sorting by `(age, name)` sorts by age, then by name among equal ages.
- Lists, rings and `Bytes` compare the same way, item by item (a byte for `Bytes`); when one runs out first, it is the smaller: `[1, 5] < [2, 3]` and `[1, 2] < [1, 2, 3]`. A list of lists, or of tuples, sorts too. The whole comparison is one `Bool`: it is not item by item as some array libraries are, where `[1, 5] < [2, 3]` would be `[true, false]`.

```kavach
print((30, "Asha") < (30, "Ravi"))     // true: equal ages, so the names decide
print([1, 5] < [2, 3])                 // true: 1 < 2 decides
print([1, 2] < [1, 2, 3])              // true: the shorter runs out first
print([[2, 1], [1, 9]].sort())         // [[1, 9], [2, 1]]
const by_age = people.sort_by(p => (p.age, p.name))
```

**Comparisons chain**, as in mathematics and Python: `a op1 b op2 c` means `a op1 b and b op2 c`, for any mix of `== != < <= > >=` and any length ([decision 0040](../decisions/0040-chained-comparisons.md)). Each operand is worked out once, left to right (3.10), and the chain stops at the first link that is false. Only neighbours are compared: `a != b != c` says nothing about `a` and `c`. Parentheses group as usual, and ranges don't chain.

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

An overflow of a type smaller than `Int` gets a help line naming both ways out, and the editor and `kavach fix` offer the first (a maintainer, 2026-11-27; *not built yet*): `help: for 254, widen first: Int(a) + Int(b); to wrap: a.wrapping_add(b)`. Arithmetic keeps its operands' type wherever it is used, stored or not, so moving `a + b` into a variable never changes its result.

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

- `print(a, b, …)` writes its values to standard output on one line, a space between them, as Python's `print` does (a maintainer, 2026-11-27). Text is written as it is, other values as 1.11 shows them, and `print()` writes an empty line. It needs the `Console` capability, which `main` has (chapter 8).
- Two named arguments change the layout: `sep:` is the text between values (`" "` by default) and `end:` the text after the last one (a line end by default; written out, it is a template, `` end: `\n` ``, as quotes take no escapes, 1.7). Both are `String`s and are written by name, after the values. On a terminal, what `print` writes shows at once, even without a line end.

```kavach
print("a =", a)                    // a = 1
print("name:", "Asha", 30, true)   // name: Asha 30 true
print(1, 2, 3, sep: ", ")          // 1, 2, 3
print("loading", end: "")          // loading, and the next print goes on this line
```

- `debug(…)` takes values, `sep:` and `end:` as `print` does. It prints them with their file and line, to standard error. It works anywhere, needs no capability, and is **removed from release builds**: `kavach run` prints it, native code (`kavach build`, `kavach run --native`) leaves it out, argument and all. So `debug(reveal(secret))` shows a secret while you develop and never in production.

## 1.11 How values are printed

`print` and `{}` in strings show a value the way you would write it in code, so output is never ambiguous:

| Value | Printed as |
|---|---|
| `Int` | `42`, `-7` |
| `Float` | Shortest form that reads back exactly, always with a `.` or an exponent: `7.0`, `0.1`, `1e21` |
| `Decimal` | All its digits, keeping its scale: `499.00` prints `499.00` |
| `Bool`, `Char` | `true`, `क` |
| `String` | The text itself; inside a collection or record it is quoted: `"Asha"`. There it shows as written, accents and other combining marks too; only `"`, `\`, line breaks (`\n`, `\r`), tabs (`\t`), and control and invisible characters (`\u{200b}`) are escaped |
| Optional | The value, or `none` |
| List, map, tuple | `[1, 2]`, `{"asha": 31}`, `(200, "OK")` |
| Record | `User(name: "Asha", age: 31)` |
| Union variant | `Paid`, `TooLarge(value: 200)` |
| Secret field | `<secret>` |

A type can choose its own form by implementing the `Display` interface (`fn display(self) -> String`). It shows in that form everywhere a value is shown: `print`, interpolation, `String(v)`, and inside lists, dicts, tuples and other records.

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
- **A dict** is taken apart by key, its pattern written like a dict literal: `"asha": a` reads key `"asha"` into `a`; a bare name is the key spelled like it (`port` reads `"port"`), as in JavaScript's `{ port }`. A key may be missing, so a name is `V?`, or `V` with a default. A bare name or `name: …` is a text key; other keys are written as literals (`7: seven`).

```kavach
const ["asha": a, "ravi": r] = ages       // a: Int?, r: Int?
const [host: server, port = 8080] = settings   // server: String?, port: Int
```

- Patterns nest: an item, a dict value or a field can be taken apart in turn, by a list, dict or record pattern in its place.

```kavach
const [first, [x, y]] = pairs             // pairs: [[Int]]; x and y are Int?
const [_, [low = 0, high = 0]] = pairs    // low and high are Int
const { home: { city } } = user           // home: Address, so city: String
const [config: [port = 8080]] = settings  // settings: {String: {String: Int}}
```

- When the part a nested pattern takes apart may be missing (a list item, a dict value, a `T?` field), every name inside it is `none` when it is, so their types are optional, unless they have defaults. A part with a default of its own (`[x, y] = [0, 0]`) is never missing.
- With `let`, the names are variables; with `const`, they never change.
- To set variables that already exist from a tuple, use tuple assignment (1.14): `(low, high) = (high, low)`.

## 1.13 Module constants

A `const` at the top level of a file, outside any function, is a **module constant**: every function in the file can read it. `pub const` lets other modules read it too, as `module.name` or through `use` (10.1).

```kavach
const count = 1_000_000
const speed: Float32 = 2.5
pub const version = "1.4"

fn step(x: Float32) -> Float32 {
    x + speed
}

fn main() {
    print(`{count} particles at {step(0.0)}`)
}
```

- A module constant the program reads is worked out once, before `main` starts, in the order the constants are written, the constants of a module a file uses before its own. Its value never changes, as any `const`'s. One that nothing reads is left out: no code and no work at start, so a panic in it never happens (and it is warned as unused, 1.4).
- Its value may use numbers, text, other module constants, and calls of functions, but no capability, and nothing that can fail without `catch` (5.3): it is worked out before `main` has run anything.

**A `const` that uses a capability** means one of two things, by whether the file has `fn main`:

| The file | `const notes = fs.read_text("notes.txt") catch ""` at the top level |
|---|---|
| Has `fn main` | An error: a module constant is worked out before `main`, which hasn't run anything yet. Work it out in `main` |
| Is a script, with no `fn main` (1.3) | One of the script's statements, run in its place, as the script's other lines are. It isn't a module constant: a function can't read it, and the error says to pass it as an argument |

```kavach
// A script: no `fn main`.
use std.fs

const notes = fs.read_text("notes.txt") catch "missing"   // a statement of the script
const limit = 3                                            // a module constant

fn show() -> String {
    `{limit}`            // fine; reading `notes` here is an error
}

print(show())
print(notes)
```

The difference keeps a script's first line, `const text = try fs.read_text(path)`, as natural as it reads, while a program with `fn main` has one kind of top-level `const` only.

- A `let` at the top level is a statement of `main` (1.3), and so is every other statement there. A function can't read it, as it is a variable of `main`: the error says so, and to make it a `const` or pass it as an argument. So is a `const` that names several values at once (`const (a, b) = pair()`).
- In a native program, a read works the value out again, giving the same value, since nothing it uses can change; the interpreter keeps it. Either way, every constant the program reads is worked out once before `main`, so a panic in one comes first.
- Its name is `snake_case`, as any value's (1.5): `const COUNT = 10` is an error that says so.
- A panic while working one out stops the program before `main` starts, with the constant's line.

## 1.14 Tuple assignment

One `=` can set several places, written as a tuple (2.3) on each side. Every value on the right is worked out first, then each place on the left is set, left to right: the one exception to the left-to-right order of [3.10](03-functions.md#310-the-order-things-are-worked-out-in). So two places swap with no temporary variable, as in Python and Go.

```kavach
let a = 1
let b = 2
(a, b) = (b, a)                          // a is 2, b is 1

let xs = [10, 20, 30]
(xs[0], xs[2]) = (xs[2], xs[0])          // [30, 20, 10]; `xs.swap(0, 2)` does the same (2.2)

let p = Point(x: 1, y: 5)
(p.x, p.y) = (p.y, p.x)                  // Point(x: 5, y: 1)

let low = 0
let high = 0
(low, high) = bounds(scores)             // any tuple value works on the right
```

- Each place is one that `=` can set alone (1.4, 1.8): a `let` variable, or a field, item, dict entry or tuple field of one. A `const` or a parameter among them is the same compile error as for `=`.
- The right side has one value for each place: a tuple written out, or any value whose type is a tuple. Another number is a compile error that gives both numbers, and each value must fit its place, as for `=`.
- Tuples nest on both sides: `((a, b), c) = ((b, a), c)`.
- The places are set in order, each worked out when it is set, as in Python: `(i, xs[i]) = (1, 9)` sets `i` first, so `xs[1]` becomes 9.
- Only `=` sets several places: `(a, b) += (1, 1)` is a compile error that says to write each on its own line.
- An item of a program's own type (`grid[k]` through `IndexSet`, 2.16) can't be a place yet: set it on its own line.
- Python writes it without parentheses, `a, b = b, a`. In Kavach that is a compile error that shows the form with them, and `xs.swap(i, j)` for two items of a list.

## Open questions

- **Tabs in the indentation of text over lines (1.7):** today a tab and a space each count as one character when the common indentation is found, so a file that mixes them may keep or remove the wrong amount. Refusing a mix, counting a tab as four spaces, or allowing only spaces are the options.
- **The same place twice in a tuple assignment (1.14):** `(a, a) = (1, 2)` sets `a` to 1, then to 2, as in Python. A compile error may be clearer.
- **Throwing a value away in a tuple assignment (1.14):** `(a, _) = pair()` is not allowed today, as `_` is not a place. Allowing it, as `const (a, _) = pair()` does, is an option.
- **List patterns in `match`:** there a pattern would test the length (`[a, b]` only for two items), unlike `const`, which never fails. Wanted, but the two meanings must not be confused.

Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
