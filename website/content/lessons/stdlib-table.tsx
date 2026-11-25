import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Tables, and CSV ----------
//
// Spec 14.34 (decision 0135): Table<R>, rows of a record type, read from
// and written to CSV, with the methods of a list of records. Every table
// comes from the spec itself, only the rows that are built
// (lib/built-spec.ts).

const SPEC = "14-standard-library/34-table.md";
const TESTS = "tests/conformance/14-standard-library";
const SRC = "stdlib/table";

/** The spec's table under a `##` heading of 14.34. */
function tableTable(heading: string) {
  const t = specTables(SPEC, "14.34").find((x) => x.heading === heading);
  if (!t) throw new Error(`spec 14.34: no table under "${heading}"`);
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const tableDoc: LessonDoc = {
  lead: "std.table gives Table<R>: rows of a record type, for data read from CSV or JSON, as pandas and R's data frames are. It has the methods of a list of records, and the compiler checks every column a program names.",
  simple: <>A table is a spreadsheet whose columns you named in a type. Because the type is known, a misspelled column is caught before the program runs, not after it gives a wrong answer.</>,
  minutes: 8,
  before: ["collections/grouping-and-statistics", "types/records"],
  body: (
    <>
      <P>
        <C>use std.table</C> brings in the module, written in Kavach in <C>stdlib/table/</C>. A type&apos;s fields
        are the columns; <C>table.parse_csv&lt;Sale&gt;</C> reads text into rows of <C>Sale</C>:
      </P>
      <Ex name="table/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>table.of(rows)</C> makes a table of a list, <C>table.read_csv&lt;R&gt;(path)</C> reads a file, and <C>table.parse_csv&lt;R&gt;(text)</C> reads text, such as the body of a web response. <Link href="/docs/standard-library/tables/csv/">CSV</Link> has every rule.</li>
        <li><C>print(t)</C> shows a grid: numbers to the right, text to the left, an empty optional as nothing, then the size.</li>
        <li>Nothing about the data is guessed: the column types come from <C>Sale</C>, so <C>month</C> is an <C>Int</C> and <C>note</C> may be <C>none</C>.</li>
      </ul>
      <H2>Rows and columns</H2>
      <Ex name="table/rows" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>t[i]</C> is row <C>i</C>, a <C>Sale</C>; out of range is a panic, as for a list. <C>t.get(i)</C> gives <C>none</C> instead.</li>
        <li>A slice, <C>t[0:2]</C>, <C>t[#2:]</C> or <C>t[::-1]</C>, is a table of those rows.</li>
        <li><C>t[&quot;amount&quot;]</C> is the column as a typed list, here a <C>[Float]</C>.</li>
        <li>A table is a value, as a list is: a copy shares its rows until one of them changes.</li>
      </ul>
      <P>A column name is checked when the program is compiled. A name that isn&apos;t a field lists the ones there are:</P>
      <Ex name="table/misspelled.fails" />
      <P>
        Each field must be something a cell can hold: a number, <C>Bool</C>, <C>Char</C>, <C>String</C>,{" "}
        <C>Decimal</C>, a <C>Date</C> or <C>DateTime</C>, a category (a union whose variants have no fields), or an
        optional of one. A list, a record or a dict is a compile error that names the field:
      </P>
      <Ex name="table/cells-only.fails" />
      <H2>The methods of a list</H2>
      <P>
        A table has every method a list of records has, with the same results (see{" "}
        <Link href="/docs/collections/grouping-and-statistics/">Grouping and statistics</Link>). So code moves from a
        list to a table with no change. Where a list&apos;s method gives a list of records, a table&apos;s gives a table:
      </P>
      <Ex name="table/methods" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>filter</C>, <C>sort_by</C>, <C>unique</C> and <C>unique_by</C> give a table; <C>group_by</C> gives a dict of tables.</li>
        <li><C>pivot</C> gives a dict of dicts, as on a list, since its columns come from the data, and a table&apos;s columns must be known when the program is compiled. Its <C>cell</C> gets the table of the rows with both keys.</li>
      </ul>
      <P>
        <C>map</C> to a record whose fields are cells gives a table; anything else gives a list. Adding a column is a{" "}
        <C>map</C> that writes every field again. An anonymous record&apos;s fields go in the order of their names, so
        the columns here are <C>amount</C>, <C>region</C>, <C>tax</C>:
      </P>
      <Ex name="table/map" />
      <P>
        <C>inner_join</C> and <C>left_join</C> take a table or a list, and give a table of the records <C>row</C>{" "}
        makes. In a <C>left_join</C>, the other row is optional, <C>none</C> when nothing matches:
      </P>
      <Ex name="table/join" />
      <P>
        <C>describe()</C> with no lambda sums up each number column, by name, in column order: count, mean, standard
        deviation, smallest, quartiles and largest. An optional column is left out. With a lambda, it sums up one
        column, as on a list:
      </P>
      <Ex name="table/describe" />
      <H2>A long table</H2>
      <P>A table of more than 10 rows prints its first 5 and its last 5, a <C>…</C> line between them, then its size:</P>
      <Ex name="table/long" />
      <H2>JSON</H2>
      <P>
        JSON goes through <Link href="/docs/standard-library/json/">std.json</Link>, with no second way.{" "}
        <C>json.decode&lt;[Sale]&gt;</C> reads an array of objects, and <C>json.stringify(t)</C> writes a table as one,
        a row each:
      </P>
      <Ex name="table/json" />
      <Note>
        Not built yet: a table keeping its columns side by side. Today it keeps its rows as one list, and{" "}
        <C>t[&quot;amount&quot;]</C> copies the column out when it is read; <C>tensor.of(t[&quot;amount&quot;])</C>{" "}
        copies it once more. Tables on the GPU are not built yet either.
      </Note>
      <H2>Every call</H2>
      {tableTable("Making and reading tables")}
      {tableTable("Rows and columns")}
      {tableTable("The same methods as a list")}
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${SPEC}`} label="spec 14.34 · std.table" />
        <SrcRef file="docs/decisions/0135-tables-and-grouping.md" label="decision 0135" />
        <SrcRef file={`${SRC}/table.kav`} line={20} symbol="type Table" />
        <SrcRef file={`${SRC}/table.kav`} line={229} symbol="fn of" />
        <SrcRef file={`${SRC}/table.kav`} line={85} symbol="fn map (to a table)" />
        <SrcRef file={`${SRC}/table.kav`} line={185} symbol="fn inner_join" />
        <SrcRef file={`${SRC}/grid.kav`} line={17} symbol="fn shown (printing)" />
        <SrcRef file={`${SRC}/grid.kav`} line={69} symbol="fn summaries (describe)" />
        <SrcRef file={`${TESTS}/table-of-a-list-has-its-rows.kav`} label="rows, tested" />
        <SrcRef file={`${TESTS}/table-a-column-is-a-typed-list.kav`} label="columns, tested" />
        <SrcRef file={`${TESTS}/table-a-misspelled-column-is-a-compile-error.kav`} label="misspelled, tested" />
        <SrcRef file={`${TESTS}/table-rows-hold-only-cells.kav`} label="cells only, tested" />
        <SrcRef file={`${TESTS}/table-slices-as-a-list-does.kav`} label="slices, tested" />
        <SrcRef file={`${TESTS}/table-has-the-methods-of-a-list.kav`} label="list methods, tested" />
        <SrcRef file={`${TESTS}/table-map-to-rows-gives-a-table.kav`} label="map, tested" />
        <SrcRef file={`${TESTS}/table-joins-take-a-table-or-a-list.kav`} label="joins, tested" />
        <SrcRef file={`${TESTS}/table-describe-summarizes-each-number-column.kav`} label="describe, tested" />
        <SrcRef file={`${TESTS}/table-prints-as-a-grid.kav`} label="printing, tested" />
        <SrcRef file={`${TESTS}/table-prints-ten-rows-at-most.kav`} label="10 rows, tested" />
        <SrcRef file={`${TESTS}/table-json-is-an-array-of-rows.kav`} label="JSON, tested" />
      </div>
    </>
  ),
};

/** CsvError's variants: what each means, and a line the samples printed. */
const ERRORS: [string, string, string][] = [
  ["Syntax(line, message)", "A quote left open, a line with more or fewer cells than the first, a column named twice, a quote inside a cell that doesn't start with one, or text after a closing quote", 'Syntax(line: 2, message: "a quote left open")'],
  ["Missing(column)", "A field with no column, no default and not optional", 'Missing(column: "units")'],
  ["Unknown(column)", "A column with no field, unless unknown: CsvUnknown.Ignore is given", 'Unknown(column: "shop")'],
  ["Value(line, column, expected, found)", "A cell that doesn't fit its field, or a row that breaks an invariant (then column is \"\" and found the row)", 'Value(line: 2, column: "units", expected: "a whole number", found: "1,299")'],
  ["File(error)", "The file itself: its FsError", 'File(error: NotFound(path: "no-such.csv"))'],
];

export const tableCsvDoc: LessonDoc = {
  lead: "table.read_csv<R>(path) and table.parse_csv<R>(text) read CSV by RFC 4180 into rows of R, strictly: a cell that doesn't fit its field fails with a CsvError naming its line and column. write_csv and to_csv write it back.",
  simple: <>A CSV file is a spreadsheet saved as text: one line per row, commas between the cells. Reading it into a type means every cell is checked once, at the door, instead of everywhere it is used.</>,
  minutes: 7,
  before: ["standard-library/tables"],
  body: (
    <>
      <P>
        The first line names the columns. A cell in double quotes may hold a comma, a line break, and <C>&quot;&quot;</C>{" "}
        for one quote. Lines end with <C>\n</C> or <C>\r\n</C>, and a byte order mark at the start is skipped:
      </P>
      <Ex name="table/csv-quotes" />
      <P>
        <C>read_csv</C> reads the file line by line, so its text is never held whole. It needs the <C>Fs</C>{" "}
        capability, as <C>write_csv</C> does; <C>write_csv</C> writes atomically, as <C>fs.write_text</C> does:
      </P>
      <Ex name="table/csv-file" />
      <H2>Columns and empty cells</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each column name matches a field exactly, and the columns may come in any order.</li>
        <li>A field with no column takes its default, or <C>none</C> if it is optional.</li>
        <li>An empty cell is <C>none</C> for an optional field, <C>&quot;&quot;</C> for a <C>String</C>, and the default for a field with one.</li>
      </ul>
      <Ex name="table/csv-empty" />
      <H2>Strict cells</H2>
      <P>
        A cell is read as <C>Int(s)</C>, <C>Float(s)</C> and <C>Decimal(s)</C> read text: <C>1,299</C> isn&apos;t a
        number, and neither is <C> 12</C> with its space. A <C>Bool</C> is <C>true</C> or <C>false</C>, a category is
        its variant&apos;s name, and a <C>Date</C> is written as ISO 8601 writes it. Each row is read straight into its
        type, cell by cell, with <Link href="/docs/standard-library/json/decoding-into-a-type/#fields-read-from-text">json.decode_texts</Link>:
      </P>
      <Ex name="table/csv-types" />
      <P>Each row is checked against the type&apos;s <C>invariant</C>s too. When no one column broke it, <C>column</C> is empty and <C>found</C> is the row:</P>
      <Ex name="table/csv-invariant" />
      <H2>Every error</H2>
      <P>Reading fails with a <C>CsvError</C>, one of five variants. Each case below is one line of this sample:</P>
      <Ex name="table/csv-errors" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Variant</th>
              <th className="py-2 pr-4 font-bold">When</th>
              <th className="py-2 font-bold">Printed</th>
            </tr>
          </thead>
          <tbody>
            {ERRORS.map(([v, when, out]) => (
              <tr key={v} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{v}</C></td>
                <td className="py-2 pr-4">{when}</td>
                <td className="py-2"><C>{out}</C></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        A column with no field is refused, as JSON refuses an unknown field. <C>unknown: CsvUnknown.Ignore</C> skips
        such columns instead:
      </P>
      <Ex name="table/csv-ignore" />
      <H2>Writing</H2>
      <P>
        Writing gives back what reading takes: a line of column names, then one line for each row. A cell is quoted
        only when it holds the separator, a quote or a line break, and <C>none</C> is an empty cell. <C>sep:</C> is
        the character between cells, for reading and writing; <C>&apos;\t&apos;</C> for tab-separated files:
      </P>
      <Ex name="table/csv-write" />
      <Ex name="table/csv-tabs" />
      <P>
        Each cell is written as <C>print</C> writes its field, so reading takes every one back: a <C>Float32</C> with its
        own shortest digits, <C>NaN</C>, <C>inf</C>, <C>-inf</C> and <C>-0.0</C>:
      </P>
      <Ex name="table/csv-floats" />
      <P>A <C>secret</C> field is written as <C>&lt;secret&gt;</C>, as JSON writes it, so a table never leaks one:</P>
      <Ex name="table/csv-secret" />
      <H2>Every type</H2>
      <P>
        <C>CsvError</C> and <C>CsvUnknown</C> come from <C>std.table</C>&apos;s <C>csv.kav</C>, and are in scope with{" "}
        <C>use std.table</C>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${SPEC}`} label="spec 14.34 · CSV" />
        <SrcRef file={`${SRC}/csv.kav`} line={16} symbol="type CsvUnknown" />
        <SrcRef file={`${SRC}/csv.kav`} line={19} symbol="type CsvError" />
        <SrcRef file={`${SRC}/csv.kav`} line={320} symbol="fn parse" />
        <SrcRef file={`${SRC}/csv.kav`} line={336} symbol="fn read" />
        <SrcRef file={`${SRC}/csv.kav`} line={364} symbol="fn write" />
        <SrcRef file={`${SRC}/table.kav`} line={234} symbol="fn read_csv" />
        <SrcRef file={`${SRC}/table.kav`} line={239} symbol="fn parse_csv" />
        <SrcRef file={`${TESTS}/csv-reads-a-header-and-quoted-cells.kav`} label="quotes, tested" />
        <SrcRef file={`${TESTS}/csv-a-quoted-cell-may-hold-a-line-break.kav`} label="line breaks, tested" />
        <SrcRef file={`${TESTS}/csv-columns-come-in-any-order.kav`} label="any order, tested" />
        <SrcRef file={`${TESTS}/csv-a-missing-column-takes-its-default.kav`} label="defaults, tested" />
        <SrcRef file={`${TESTS}/csv-an-empty-cell-is-none-or-empty-text.kav`} label="empty cells, tested" />
        <SrcRef file={`${TESTS}/csv-an-unknown-column-is-refused.kav`} label="unknown columns, tested" />
        <SrcRef file={`${TESTS}/csv-cells-are-read-strictly.kav`} label="strict cells, tested" />
        <SrcRef file={`${TESTS}/csv-reads-categories-and-dates.kav`} label="categories and dates, tested" />
        <SrcRef file={`${TESTS}/csv-rows-are-checked-against-invariants.kav`} label="invariants, tested" />
        <SrcRef file={`${TESTS}/csv-a-quote-left-open-is-a-syntax-error.kav`} label="Syntax, tested" />
        <SrcRef file={`${TESTS}/csv-tabs-separate-with-sep.kav`} label="sep, tested" />
        <SrcRef file={`${TESTS}/csv-written-cells-are-quoted-only-when-needed.kav`} label="writing, tested" />
        <SrcRef file={`${TESTS}/csv-round-trips-through-a-file.kav`} label="files, tested" />
      </div>
    </>
  ),
};
