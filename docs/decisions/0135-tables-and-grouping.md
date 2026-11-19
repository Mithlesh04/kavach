# 0135. Tables and grouping, for researchers

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

Researchers and data scientists come to Kavach from pandas and polars in Python, and from R's dplyr. Their daily work is on tables: read a CSV file, group the rows by a key, count and add up per group, summarize a column, join two tables on a key, pivot, and write the result out.

Before this decision, Kavach lists had `map`, `filter`, `sum`, `count`, `sort_by` and `find`, and dicts only their basic methods. JSON decoded into typed records (2.12). Nothing grouped, joined or pivoted, and nothing read CSV. `std.tensor` (14.33) is for numbers in many dimensions, not for rows with named columns of different types.

A maintainer chose on 2026-11-19 (to-do 27): **both, methods first.** First pandas-like methods on typed lists of records and on dicts, checked when the program is compiled; then `std.table`, a table kept as columns for big data, read from and written to CSV and JSON; and both on the GPU too, with the same results as on the CPU.

## Research

| Job | pandas | polars | dplyr | SQL | Kavach |
|---|---|---|---|---|---|
| Group rows | `df.groupby("r")` | `df.group_by("r")` | `group_by(r)` | `GROUP BY r` | `xs.group_by(.r)` |
| Count per key | `value_counts()`, `groupby().size()` | `.len()` | `count(r)` | `COUNT(*)` | `xs.count_by(.r)` |
| Sum per key | `groupby("r")["a"].sum()` | `.agg(pl.col("a").sum())` | `summarise(sum(a))` | `SUM(a)` | `xs.group_by(.r).map_values(g => g.sum(.a))` |
| Mean, median, std | `mean()`, `median()`, `std()` | the same | `mean(a)` and others | `AVG(a)` | `xs.mean(.a)`, `median`, `std`, `var` |
| Summary | `describe()` | `describe()` | `summary()` | – | `xs.describe(.a)` |
| Row with the largest value | `df.loc[df.a.idxmax()]` | `.top_k(1)` | `slice_max(a)` | `ORDER BY … LIMIT 1` | `xs.max_by(.a)` |
| Each once | `drop_duplicates()`, `unique()` | `unique()` | `distinct()` | `DISTINCT` | `xs.unique()`, `xs.unique_by(.r)` |
| Join | `merge(how="inner")` | `join(how="inner")` | `inner_join`, `left_join` | `INNER JOIN`, `LEFT JOIN` | `inner_join`, `left_join` |
| Pivot | `pivot_table()` | `pivot()` | `pivot_wider()` | – | `xs.pivot(.r, .c, cell)` |
| Pick columns | `df[["a", "b"]]` | `select()` | `select()` | `SELECT a, b` | `xs.map(x => (a: x.a, b: x.b))` |
| Read and write CSV | `read_csv`, `to_csv` | `read_csv`, `write_csv` | `read_csv` (readr) | – | `table.read_csv<R>`, `t.write_csv` |

What we took from each:

- **pandas** names the jobs researchers know: `describe`, `pivot`, `mean`, `median`, `std`. Its quartiles interpolate linearly, and its `std` divides by the count minus 1; we do the same, so numbers match theirs. pandas finds columns by text when the program runs, so a misspelled column fails late, and its many ways to do one job (`df.a`, `df["a"]`, `df.loc`, `df.get`) are what we avoid.
- **polars** keeps data as columns and plans work before running it. `std.table` keeps columns for the same reason: one column is read without the others, and number columns go to the GPU as they are.
- **dplyr** names each kind of join (`inner_join`, `left_join`) instead of passing a `how` argument. We do the same, which also keeps `join` free: it already joins text (14.2).
- **SQL** defines what a join gives when keys repeat: every pair. Our joins do the same.
- **Ruby, Kotlin and Lodash** have `group_by`, `count_by`, `min_by` and `max_by` on lists. Lodash's `sumBy` and Kotlin's `sumBy` add `f(x)` over the whole list, with no groups: the job of Kavach's `sum(f)`.

## Decision

```kavach
const per_region = sales.group_by(.region).map_values(g => g.sum(.amount))
const counts = sales.count_by(.region)
const top = sales.max_by(.amount)
const summary = sales.describe(.amount)
const named = orders.inner_join(users, .user_id, .id, (o, u) => (name: u.name, total: o.total))

const big = try table.read_csv<Sale>("sales.csv")
print(big.filter(.amount > 90.0).mean(.amount))
```

1. **Methods on lists of records, and on dicts (14.2).** Every key and column is a lambda, so the compiler checks it. The set, and why each is there:
   - `group_by(key) -> {K: [T]}`: the base of every grouped job.
   - `count_by(key) -> {K: Int}`: the most common grouped job (pandas' `value_counts`). It replaces `collections.counts`, so the job keeps one name.
   - `map_values(f)` on dicts: turns groups into one value each, so any reduction works per key without a method for each.
   - `to_list()` on dicts: the pairs as a list, so `sort_by` and slicing apply, for "the top 10 keys".
   - `unique()` and `unique_by(key)`: drop repeated rows, or rows with a repeated key. `unique()` replaces `collections.unique`.
   - `min_by(f)` and `max_by(f)`: the row with the smallest or largest value, not just the value.
   - `mean`, `median`, `var`, `std`: the statistics asked for most. `mean` replaces `math.mean`.
   - `describe(f)`: count, mean, std, min, quartiles and max in one call, as pandas gives them.
   - `inner_join` and `left_join` with a `row` lambda: the joins used most. The `row` lambda builds each joined row as a flat record, so a join of tables is a table, and lists and tables have one signature.
   - `pivot(row_key, column_key, cell)`: the name researchers look for. It is defined as two `group_by`s, so it adds no new rule.
2. **No `sum_by`.** The to-do named it, but `xs.sum_by(.region, .amount)` takes two lambdas whose roles a reader must remember, and Lodash and Kotlin use `sumBy` for a plain sum. A sum per key is `group_by(key).map_values(g => g.sum(f))`, which the compiler runs as one pass, and on the GPU without making the groups (15.5). `sum_by` is a compile error that names both forms.
3. **No `select`.** `map` to an anonymous record (2.14) picks columns out already.
4. **Names from other tools are compile errors** that name Kavach's: `value_counts`, `distinct`, `drop_duplicates`, `merge`, `select`, `average`, `avg`, `sum_by`, and `join` given lambdas.
5. **Numbers.** A maintainer chose on 2026-11-19:
   - **`sum` and `mean` of floats (`Float`, `Float32`) are exact sums, rounded once.** So they give the same bits in any order: on one core, on all cores and on the GPU. This changes [decision 0031](0031-loops-on-all-cores.md)'s rule that a list's float sum keeps its order and stays on one core. A `+` written in the code still keeps its order ([decision 0122](0122-floats-give-the-same-bits-everywhere.md)), and so does a float `+=` in a loop. `mean` of whole numbers and `Decimal`s is exact too, rounded once.
   - **`var` and `std` divide by the count minus `correction:`**, 1 unless given, as pandas and R do; `correction: 0` is NumPy's population form. Tensors (to-do 28) take the same argument.
   - Quartiles interpolate linearly. NaN is never skipped without saying; a program filters first.
6. **`std.table` (14.34).** `Table<R>`, where `R` is a record whose fields are columns: numbers, `Bool`, `Char`, `String`, `Decimal`, dates, categories, and optionals of them. It has every method a list of records has, with the same results, plus `t["name"]` for a column, slicing as lists slice (to-do 26), CSV reading and writing by RFC 4180, strict as JSON decoding is, and printing as a grid. JSON goes through `std.json`, with no second way.
7. **On the GPU (15.5).** Grouping, counts, whole-number sums and means, `min_by`, `max_by`, `median`, sorting and joins of columns of `Float32`, `Int32`, `UInt32` and `Bool` run there, with the results and order of the CPU. Sums and means of `Float32` run there too, since an exact sum doesn't depend on the order of adding.

## Consequences

- A researcher's common jobs are one line each, and every column name is checked before the program runs.
- `collections.unique`, `collections.counts` and `math.mean` are removed, with compile errors that name the methods; their conformance tests and the website's samples change with the code.
- A table must have a record type before a CSV is read. This costs a few lines for a quick look at an unknown file (an open question in 14.34).
- The compiler must recognize `group_by(…).map_values(…)` with a reduction as one pass, on the CPU and the GPU, so the groups are never made.
- The checker, the native runtime and `std.table` need the new methods; the GPU work needs grouped reductions, sorting and hash joins in WGSL.
- `sum` of floats changes for every list, not only tables: the interpreter, the native runtime and the GPU need an exact sum (a wide fixed-point accumulator), and a float `sum` may now use all cores. A test that expects an in-order float sum's rounding changes with the code. `tensor.sum()` follows the same rule (14.33).

## Alternatives considered

- **A pandas-style `DataFrame` with columns found by text.** Familiar, and good for a quick look, but a misspelled column fails only when the program runs, and every value is untyped. Rejected for typed records; a quick look is an open question.
- **Only `std.table`, no list methods.** Small lists of records (a few hundred rows from JSON) would need a table first. Rejected: the methods cost little, and a table then has nothing new to learn.
- **Only list methods, no table.** A list of records keeps each row together, so reading one column of a big file touches all of it, and columns can't go to the GPU as they are. Rejected for big data.
- **`sum_by`, `mean_by` and the like, one method per reduction per key.** Many names, each with two lambdas. Rejected for `group_by` and `map_values`.
- **One `join(other, on, how)`.** `join` already joins text, and `how: "left"` is text the compiler can't check. Rejected for `inner_join` and `left_join`.
- **Joins giving pairs `(T, U)`.** Simple for lists, but a pair of records can't be a table's row. Rejected for a `row` lambda, the same on both.
- **`std` dividing by the count, as NumPy does.** Rejected as the default for the count minus 1, as pandas, R and spreadsheets do, since researchers compare with them; `correction: 0` gives NumPy's form.
- **Float sums in order (decision 0031), or a reordered `fast_sum`.** In order runs on one core and never on the GPU; a reordered sum gives different bits on different machines. An exact sum costs a little more per item, but gives one answer everywhere, and a more accurate one.
- **Skipping NaN and `none` in statistics, as pandas does.** Convenient, but values vanish without a word. Rejected for now; a lambda that gives `N?` is an open question in 14.34.
- **The name `DataFrame`.** Longer, and a term of art; `Table` says what it is.
