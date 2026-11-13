# 10.15 Tagged literals

A package can give text a meaning the compiler checks: a **tagged literal**. `sql"select id from orders where owner = $1"` is a `Query`, and the package's checker reads the text when the program is compiled, so a misspelled keyword is a compile error at its column, not a failure in production ([decision 0100](../../decisions/0100-tagged-literals.md)).

```kavach
// In package pg:
pub type Query {
    text: String                // not pub: only the literal and pg itself make a Query
}

/// SQL for PostgreSQL, checked when the program is compiled.
pub literal sql: Query check check_sql language "sql"

fn check_sql(text: String) -> [LiteralProblem] { … }

// In a program:
use pg.{sql}

const recent = sql"select id, item from orders where owner = $1"
const bad = sql"selct id from orders"      // compile error at `selct`, from check_sql
```

- **Declaring a tag.** `literal name: T check f` declares the tag `name`. `T` is a record whose first field is a `String`: the literal's value is a `T` holding its text, its other fields taking their defaults. `f` takes the text and gives `[LiteralProblem]`. It must be pure, with no `uses` (8.1), as it runs while the program is compiled. `language "sql"` is optional: it tells editors which language to colour inside the literal.
- **Writing one.** The tag's name is written right before the quote, with no space: `sql"…"`, or `pg.sql"…"` through the package. Three quotes let a literal span lines, as other text does (1.7): `sql"""…"""`.
- **The text is raw.** Inside a tagged literal, `\` and `{` are characters like any other: no escapes and no interpolation, so values can't be pasted into it, and `at` counts characters of the text as written. Values go in separately, as `$1` does in SQL.
- **Checking.** The compiler runs `f` on each literal's text. Each problem is reported at its place in the literal: an error, or a warning when its `warning` is true. In a literal over lines, `at` counts characters of the text the checker gets, with the common indentation removed, and the problem is shown on its own line in the source. A checker that panics or runs longer than a second is a compile error that names the tag.
- **Only a literal makes one.** The record's text field isn't `pub`, so outside its package a `T` comes only from a literal (10.1); inside, its own code may still build one. A function that makes a `T` from text known only when the program runs (migrations read from files, a query builder) is marked `audited fn`, and `kavach audit` lists each call of it, as it lists `reveal` and `unsafe`.
- **Tests.** Generated tests (11.4) don't make a `T` from made-up text: a function that takes one is tested through its `example` clauses and `test` blocks, which write literals.
- **Joining.** `sql"…" + sql"…"` is not part of this version.

```kavach
/// One problem a tag's checker found (built in, every program has it).
pub type LiteralProblem {
    pub at: Int                // a character index into the text
    pub length: Int = 1
    pub message: String
    pub hint: String = ""
    pub warning: Bool = false
}
```
