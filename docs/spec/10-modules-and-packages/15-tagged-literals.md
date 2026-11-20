# 10.15 Tagged literals

A package can give text a meaning the compiler checks: a **tagged literal**. `sql"select id from orders where owner = $1"` is a `Query`, and the package's checker reads the text when the program is compiled, so a misspelled keyword is a compile error at its column, not a failure in production ([decision 0100](../../decisions/0100-tagged-literals.md)).

```kavach
// In package pg:
pub(out) type Query {
    text: String                // not pub: only the literal and pg itself make a Query
    params: [Json] = []         // the values of its `{…}`, in order
}

/// SQL for PostgreSQL, checked when the program is compiled.
pub(out) literal sql: Query check check_sql param pg_param language "sql"

fn check_sql(text: String) -> [LiteralProblem] { … }

/// How PostgreSQL writes the n-th parameter: `$1`, `$2`, ….
fn pg_param(n: Int) -> String {
    `${n}`
}

// In a program:
use pg.{sql}

const recent = sql"select id, item from orders where owner = $1"
const mine = sql`select id, item from orders where owner = {owner}`
// mine.text is "select id, item from orders where owner = $1"; mine.params holds owner
const bad = sql"selct id from orders"      // compile error at `selct`, from check_sql
```

- **Declaring a tag.** `literal name: T check f` declares the tag `name`. `T` is a record whose first field is a `String`: the literal's value is a `T` holding its text, its other fields taking their defaults. `f` takes the text and gives `[LiteralProblem]`. It must be pure, with no `uses` (8.1), as it runs while the program is compiled. `param p` is optional: the tag then takes parameters (below). `language "sql"` is optional: it tells editors which language to colour inside the literal.
- **Writing one.** The tag's name is written right before the quote or backtick, with no space: `sql"…"`, `` sql`…` ``, or `pg.sql"…"` through the package. Three quotes let a literal span lines, as other text does (1.7): `sql"""…"""`.
- **Text in quotes is raw.** Inside `sql"…"`, `\` and `{` are characters like any other: no escapes and no interpolation, and `at` counts characters of the text as written. Values go in separately, as `$1` does in SQL.
- **A tagged template.** `` sql`…` `` is a template (1.7, [decision 0131](../../decisions/0131-plain-strings-and-backtick-templates.md)): escapes are read (`\n`, `\{` for a brace, `` \` `` for a backtick), and it spans lines as a template does, when its text starts on the line after the backtick. Each `{…}` in it is a **parameter**: its value is never pasted into the text, so a value can't change what the text means ([decision 0142](../../decisions/0142-tagged-template-parameters.md)).
- **Placeholders.** `param p` names the function that writes the n-th parameter in the text, counting from 1: `p` must be `fn p(n: Int) -> String`, pure, with no `!` and no `uses`, like the checker. In the literal's text, the n-th `{…}` is replaced by `p(n)`: PostgreSQL's `pg_param` gives `$1`, `$2`, …, and SQLite's gives `?`. So `` sql`… where owner = {owner} and kind = {kind}` `` has the text `… where owner = $1 and kind = $2`.
- **Parameter values.** A tag with `param` makes a record with a field `params: [P]`, which holds the parameters' values in the order they are written; `P` is the one type they all become. Each value is converted to a `P`, checked when the program is compiled:
  - a value that is a `P` stays as it is;
  - when `P` is `Json`, any value converts as an item of a `Json` list does (`json.of`, 2.12);
  - when `P` is a union with exactly one variant whose only field has the value's type, the value becomes that variant: with `type Param = Text(value: String) | Number(value: Int)`, `{owner}` of type `String` becomes `Param.Text(value: owner)`.

  Any other value is a compile error at its `{…}`. A literal with no `{…}`, in quotes or backticks, has `params: []`.
- **A tag without `param` refuses `{…}`.** Its tagged template may still use escapes, but a `{…}` in it is a compile error that says the tag takes no parameters and to write `\{` for a brace.

```kavach
use pg.{sql}

fn orders_of(db: Client, owner: String, newest: Int) -> [Order] ! PgError uses Net {
    // The text is "select id, item from orders where owner = $1 and id > $2";
    // owner and newest travel apart, as `Json` values.
    try db.query<Order>(sql`select id, item from orders where owner = {owner} and id > {newest}`)
}
```

- **Checking.** The compiler runs `f` on each literal's text. Each problem is reported at its place in the literal: an error, or a warning when its `warning` is true. In a literal over lines, `at` counts characters of the text the checker gets, with the common indentation removed, and the problem is shown on its own line in the source. In a tagged template, the checker gets the text with the placeholders in, so it checks what the database gets: each escape counts as the one character it stands for, and a problem inside a placeholder is shown at its `{…}`. A checker or `param` function that panics or runs longer than a second is a compile error that names the tag.
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
