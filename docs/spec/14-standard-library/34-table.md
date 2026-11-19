# 14.34 std.table

*Not built yet.*

A `Table<R>` holds rows of the record type `R`, kept as columns: each field's values side by side. It is for big data read from CSV or JSON, as pandas, polars and R's data frames are ([decision 0135](../../decisions/0135-tables-and-grouping.md)). It has the same methods as a list of records (14.2), so code moves from a list to a table with no change. Its columns are the fields of `R`, so the compiler checks every column a program names.

```kavach
use std.table

type Sale {
    region: String
    month: Int
    amount: Float
    note: String?
}

fn main() ! CsvError uses Fs, Console {
    const sales = try table.read_csv<Sale>("sales.csv")
    print(sales.len())                                     // 3
    print(sales["amount"])                                 // [120.0, 80.0, 100.0]
    print(sales.group_by(.region).map_values(g => g.sum(.amount)))   // {"north": 220.0, "south": 80.0}
    print(sales.describe())                                // a summary of each number column
    const big = sales.filter(.amount > 90.0)
    try big.write_csv("big.csv")
}
```

With `sales.csv`:

```
region,month,amount,note
north,1,120.0,
south,1,80.0,"late, paid in cash"
north,2,100.0,
```

## Making and reading tables

| Call | Gives |
|---|---|
| `table.of(rows) -> Table<R>` | A table of the list `rows` |
| `table.read_csv<R>(path) -> Table<R> ! CsvError` | The rows of the CSV file at `path`, read as the file is read, so the file's text is never held whole |
| `table.parse_csv<R>(text) -> Table<R> ! CsvError` | The same, from text, such as the body of a web response |
| `t.write_csv(path) ! CsvError` | Writes the table as CSV, atomically, as `fs.write_text` does (14.3) |
| `t.to_csv() -> String` | The table as CSV text |

- `read_csv` and `write_csv` need the `Fs` capability (`Fs.read` and `Fs.write`, 8.1), as `std.fs` does.
- `read_csv` and `parse_csv` take `sep:`, the `Char` between cells: `','` unless given, `'\t'` for tab-separated files. They take `unknown: CsvUnknown.Ignore` to skip columns `R` has no field for (below). `write_csv` and `to_csv` take `sep:` too.
- **JSON** goes through `std.json` (2.12), with no second way: `table.of(try json.decode<[Sale]>(text))` reads an array of objects, and `json.stringify(t)` writes the table as one, a row each, as it writes a list of records.

## Rows and columns

`R` is a record type, declared (2.4) or anonymous (2.14). Each field is a column, in the order the fields are written. A field's type must be one a cell can hold:

- a number type, `Bool`, `Char`, `String` or `Decimal`;
- `time.Date` or `time.DateTime` (14.4);
- a union whose variants have no fields, such as `type Region = North | South`: a category, kept as small numbers;
- an optional of one of these, `T?`, for a cell that may be empty.

Any other field type (a list, a record, a dict) is a compile error that names the field.

| Method | Gives |
|---|---|
| `t.len()` | How many rows it has |
| `t.columns() -> [String]` | The names of its columns, in order |
| `t[i]` | Row `i`, as an `R`; out of range is a panic, as for a list (2.2) |
| `t.get(i)` | Row `i`, or `none` |
| `t[a:b:step]` | A table of those rows, as a list is sliced (2.19): `t[0:10]`, `t[#5:]`, `t[::2]` |
| `t["name"]` | The column `name`, as a list: `sales["amount"]` is a `[Float]` |
| `t.rows() -> [R]` | Every row, as a list |

- `t["name"]` takes a name written in the code. A name that is not a field of `R` is a compile error that names the fields there are. The list shares the table's storage, so reading a column copies nothing.
- A column of numbers is a tensor's items with no copy: `tensor.of(t["amount"])` (14.33).
- A table is a value, as a list is (6.1): a copy shares its columns until one of them changes.

```kavach
const first = sales[0]                  // a Sale
const last_two = sales[#2:]             // a Table<Sale> of the last two rows
const amounts = sales["amount"]         // [Float]
const wrong = sales["amont"]            // error: `Sale` has no field `amont`; it has region, month, amount, note
```

## The same methods as a list

A table has every method a list of records has, with the same meaning, the same order of results and the same rules for keys and numbers (14.2): `filter`, `map`, `sort_by`, `find`, `any`, `all`, `count`, `sum`, `for_each`, `group_by`, `count_by`, `unique`, `unique_by`, `min_by`, `max_by`, `mean`, `median`, `var`, `std`, `describe`, `inner_join`, `left_join` and `pivot`.

Where a list's method gives a list of records, a table's gives a table:

| Method | Gives, on a table |
|---|---|
| `t.filter(test)`, `t.sort_by(key)`, `t.unique()`, `t.unique_by(key)` | A `Table<R>` |
| `t.map(f)` | A `Table<S>` when `f` gives a record `S` whose fields a cell can hold; otherwise a list, as on a list |
| `t.group_by(key)` | A `{K: Table<R>}` |
| `t.inner_join(other, key, other_key, row)`, `t.left_join(…)` | A `Table<S>` of the records `row` makes; `other` is a table or a list |
| `t.describe()` | With no lambda: a `{String: Summary}`, the summary (14.2) of each number column, in column order |

`pivot` gives a dict of dicts, as on a list: its columns come from the data, so their names are not known when the program is compiled, and a table's columns must be.

A row picked out of a join is a flat record, so a join's result is a table too:

```kavach
const named = orders.inner_join(users, .user_id, .id, (o, u) => (name: u.name, total: o.total))
print(named.sum(.total))
```

Adding a column makes a new record with every field: `t.map(s => (region: s.region, amount: s.amount, tax: s.amount * 0.18))`.

## CSV

Reading follows RFC 4180:

- Cells are separated by `sep`. A cell in double quotes may hold `sep`, line breaks, and `""` for one quote. Lines end with `\n` or `\r\n`. The text is UTF-8; a byte order mark at the start is skipped.
- The first line names the columns. Each name matches a field of `R` exactly; the columns may come in any order.
- A field with no column takes its default, or `none` if it is optional; otherwise reading fails with `Missing`. A column with no field fails with `Unknown`, as JSON refuses unknown fields ([decision 0096](../../decisions/0096-unknown-fields-are-refused.md)), unless `unknown: CsvUnknown.Ignore` is given.
- A cell is read strictly, as `Int(s)`, `Float(s)` and `Decimal(s)` read text (2.1): `1,299` is not a number, and neither is ` 12`. A `Bool` is `true` or `false`; a category its variant's name; a `Date` and a `DateTime` as ISO 8601 writes them (14.4). A cell that doesn't fit fails with `Value`, naming its line and column.
- An empty cell is `none` for an optional field, `""` for a `String`, and the default for a field with one. Anywhere else it fails with `Value`.
- Each row is checked against `R`'s `invariant`s (7.3); a row that breaks one fails with `Value`, its `expected` the invariant, as a JSON decode does (2.12).

Writing gives back what reading takes: a line of column names, then one line for each row, each ending with `\n`. A cell is quoted only when it holds `sep`, a quote or a line break. Numbers are written as `print` writes them, `none` as an empty cell, and a `secret` field as `<secret>`, as JSON writes it ([decision 0132](../../decisions/0132-json-for-every-value.md)).

```kavach
pub type CsvUnknown = Refuse | Ignore

pub type CsvError = File(error: FsError) |
    Syntax(line: Int, message: String) |
    Missing(column: String) |
    Unknown(column: String) |
    Value(line: Int, column: String, expected: String, found: String)
```

`Syntax` is a quote left open, or a line with more or fewer cells than the first. `File` holds the error of the file itself (14.3).

## Printing

`print(t)` writes the column names, then the rows, each column as wide as its widest cell: numbers to the right, everything else to the left. A table of more than 10 rows shows its first 5 and last 5, with a `…` line between, and then its size:

```
region  month  amount  note
north       1   120.0
south       1    80.0  late, paid in cash
north       2   100.0
[3 rows × 4 columns]
```

## On the GPU

A table keeps its columns side by side, which is how a GPU reads data, so a column of `Float32`, `Int32`, `UInt32` or `Bool` goes to the GPU with no packing. Its methods run there under the rules of 15.5, as a list's do: `filter`, `map`, `count_by`, `group_by` followed by `map_values` of a sum, count, minimum or maximum, `unique`, `sort_by`, `min_by`, `max_by`, `median` and the joins.

- Only the columns the lambdas read go to the GPU. A `filter` that tests the number columns of a table that also has text runs on the GPU; the rows it keeps are then gathered from the other columns on the CPU, by their positions.
- The results, and their order, are those the CPU gives (15.5). `KAVACH_GPU=check` (15.2) checks them as it checks a `map`.
- Columns of `Float` and `Int` (64 bits), text, dates and `Decimal` stay on the CPU, as 15.1 says.
- A column a GPU step made stays in GPU memory until the CPU reads it ([decision 0055](../../decisions/0055-gpu-results-stay-until-read.md)), so a chain of steps copies only its result back.

## Open questions

- **Missing values in statistics.** Should `mean`, `median`, `var`, `std` and `describe` take a lambda that gives `N?` and leave out the `none`s, as pandas skips NaN by default? Today a program writes `t.filter(.price != none)` first, and then still needs `?? 0.0` in the lambda.
- **Column names that aren't names.** A CSV header such as `Unit Price` can't be a field name. Options: a rename at the field (`unit_price: Float from "Unit Price"`), a `columns:` argument to `read_csv`, or turning each such name into `snake_case`.
- **Exploring an unknown file.** Reading a CSV whose columns aren't known yet needs a type first. Options: `kavach table schema data.csv` printing the `type` to paste in, or an untyped `Table<Json>`.
- **Lenient reading.** Spaces around a number, `True` and `1` for `Bool`, `NA` and `NaN` for an empty cell, numbers written `1,299`: refused today; perhaps as named options of `read_csv`.
- **More formats.** Parquet and Arrow need an outside library (a decision record first); JSON Lines, one object per line, may fit `std.json`.
- **Adding a column** without writing every field again: a record spread such as `(…s, tax: s.amount * 0.18)`.
- **One way to read a column.** `t["amount"]` and `t.map(.amount)` give equal lists. Should `t.map(.amount)` stay, or be a compile error that names `t["amount"]`?
- **Full outer joins and joins on several keys.** A tuple key does several keys (`(o.day, o.shop)`); a full outer join has no method yet.
- **Rolling windows and ranks** (`rolling(7).mean(...)`, `rank`), time series resampling, and cumulative sums, which researchers use often.
- **Using every core.** Reading a large CSV, and grouping, on all cores with the same result as in order (decision 0031).
