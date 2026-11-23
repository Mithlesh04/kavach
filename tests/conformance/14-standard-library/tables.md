# 14. Standard library: tables

Conformance tests for std.table (14.34): tables of records, their columns and methods, and CSV read and written. Part of the [14-standard-library](README.md) list; format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 14.34 | [table-a-column-is-a-typed-list.kav](table-a-column-is-a-typed-list.kav) | `t["name"]` is the column `name` as a list of its field's type. | runs |
| 14.34 | [table-a-misspelled-column-is-a-compile-error.kav](table-a-misspelled-column-is-a-compile-error.kav) | `t["name"]` with a name that is not a field of the row type is a compile error that names the fields there are. | compile error |
| 14.34 | [table-describe-summarizes-each-number-column.kav](table-describe-summarizes-each-number-column.kav) | `t.describe()` with no lambda gives the summary of each number column, by its name, in column order. | runs |
| 14.34 | [table-has-the-methods-of-a-list.kav](table-has-the-methods-of-a-list.kav) | a table has the methods of a list of records, with the same results; where a list's gives a list of records, a table's gives a table. | runs |
| 14.34 | [table-joins-take-a-table-or-a-list.kav](table-joins-take-a-table-or-a-list.kav) | a table's `inner_join` and `left_join` take a table or a list, and give a table of the rows `row` makes. | runs |
| 14.34 | [table-json-is-an-array-of-rows.kav](table-json-is-an-array-of-rows.kav) | JSON goes through `std.json`: `json.stringify(t)` writes a table as an array of rows, and `table.of(json.decode<[R]>(text))` reads one. | runs |
| 14.34 | [table-map-to-rows-gives-a-table.kav](table-map-to-rows-gives-a-table.kav) | `t.map(f)` gives a table when `f` makes a record whose fields are cells, and a list otherwise. | runs |
| 14.34 | [table-of-a-list-has-its-rows.kav](table-of-a-list-has-its-rows.kav) | `table.of(rows)` makes a table of a list of records: `len`, `columns`, `t[i]`, `t[#k]`, `get` and `rows` read it as a list is read. | runs |
| 14.34 | [table-prints-as-a-grid.kav](table-prints-as-a-grid.kav) | `print(t)` writes the column names, then the rows, each column as wide as its widest cell, numbers to the right, and then its size. | runs |
| 14.34 | [table-prints-ten-rows-at-most.kav](table-prints-ten-rows-at-most.kav) | a printed table of more than 10 rows shows its first 5 and last 5, with a `…` line between, then its size. | runs |
| 14.34 | [table-rows-hold-only-cells.kav](table-rows-hold-only-cells.kav) | a table's row type may only have fields a cell can hold; a list field is a compile error that names the field. | compile error |
| 14.34 | [table-slices-as-a-list-does.kav](table-slices-as-a-list-does.kav) | `t[a:b:step]` gives a table of those rows, as a list is sliced. | runs |
| 14.34 | [csv-a-missing-column-takes-its-default.kav](csv-a-missing-column-takes-its-default.kav) | a field with no CSV column takes its default, or `none` if it is optional; otherwise reading fails with `Missing`. | runs |
| 14.34 | [csv-an-empty-cell-is-none-or-empty-text.kav](csv-an-empty-cell-is-none-or-empty-text.kav) | an empty CSV cell is `none` for an optional field, `""` for a `String`, and the default for a field with one; anywhere else it fails with `Value`. | runs |
| 14.34 | [csv-an-unknown-column-is-refused.kav](csv-an-unknown-column-is-refused.kav) | a CSV column with no field fails with `Unknown`, unless `unknown: CsvUnknown.Ignore` is given. | runs |
| 14.34 | [csv-a-quoted-cell-may-hold-a-line-break.kav](csv-a-quoted-cell-may-hold-a-line-break.kav) | a quoted CSV cell may hold line breaks; lines may end with `\n` or `\r\n`, and a byte order mark at the start is skipped. | runs |
| 14.34 | [csv-a-quote-left-open-is-a-syntax-error.kav](csv-a-quote-left-open-is-a-syntax-error.kav) | a quote left open, or a line with more or fewer cells than the first, fails reading a CSV with `Syntax`. | runs |
| 14.34 | [csv-cells-are-read-strictly.kav](csv-cells-are-read-strictly.kav) | a CSV cell is read strictly, as `Int(s)` and `Float(s)` read text: a cell that doesn't fit fails with `Value`, naming its line and column. | runs |
| 14.34 | [csv-columns-come-in-any-order.kav](csv-columns-come-in-any-order.kav) | each CSV column name matches a field exactly, and the columns may come in any order. | runs |
| 14.34 | [csv-reads-a-header-and-quoted-cells.kav](csv-reads-a-header-and-quoted-cells.kav) | CSV is read by RFC 4180: the first line names the columns, a quoted cell may hold the separator and `""` for a quote, and an empty cell is `none` for an optional field. | runs |
| 14.34 | [csv-reads-categories-and-dates.kav](csv-reads-categories-and-dates.kav) | a CSV cell of a category is its variant's name, and a `Date` is read as ISO 8601 writes it; anything else fails with `Value`. | runs |
| 14.34 | [csv-round-trips-through-a-file.kav](csv-round-trips-through-a-file.kav) | `write_csv` writes a table to a file and `read_csv` reads it back, the file read line by line; both need the `Fs` capability. | runs |
| 14.34 | [csv-rows-are-checked-against-invariants.kav](csv-rows-are-checked-against-invariants.kav) | each CSV row is checked against its type's invariants; a row that breaks one fails with `Value`, its `found` the row when no one column broke it. | runs |
| 14.34 | [csv-tabs-separate-with-sep.kav](csv-tabs-separate-with-sep.kav) | `sep:` is the character between CSV cells, `','` unless given; reading and writing take it. | runs |
| 14.34 | [csv-written-cells-are-quoted-only-when-needed.kav](csv-written-cells-are-quoted-only-when-needed.kav) | writing CSV gives a line of column names, then a line for each row; a cell is quoted only when it holds the separator, a quote or a line break, and `none` is an empty cell. | runs |
| 14.34 | [csv-cells-are-written-as-print-writes-them.kav](csv-cells-are-written-as-print-writes-them.kav) | a CSV cell is written as `print` writes its field: a `Float32`'s own digits, `NaN`, `inf` and `-0.0`; reading takes them back. | runs |
