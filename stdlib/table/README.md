# stdlib/table/

`std.table` ([spec 14.34](../../docs/spec/14-standard-library/34-table.md), [decision 0135](../../docs/decisions/0135-tables-and-grouping.md)): `Table<R>`, rows of a record type for big data, read from and written to CSV, with the methods of a list of records (14.2). Written in Kavach; the compiler adds the rules that need types (`compiler/checker/src/std_table.rs`, `compiler/driver/src/tables.rs`).

| File | Module | Contents |
|---|---|---|
| `table.kav` | `std.table` | `Table<R>` and its methods; `table.of`, `table.read_csv` and `table.parse_csv` |
| `csv.kav` | `std.table.csv` | `CsvError` and `CsvUnknown`; a row type's columns from its JSON Schema; reading CSV line by line by RFC 4180, and writing it |
| `grid.kav` | `std.table.grid` | A table printed as a grid, and the summary of each number column (`t.describe()`) |
