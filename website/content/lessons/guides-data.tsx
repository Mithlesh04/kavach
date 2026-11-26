import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Read and Write Data Files ----------
//
// One task from start to finish: guides/sales-report.project reads sales
// from a CSV into typed rows, totals them per item, writes a JSON report
// and reads it back. guides/sales-first.project is the same program
// before the fix the generated tests asked for: `units: Int`, whose test
// run finds the overflow.

const report = guideProject("sales-report", ["src/main.kav", "sales.csv", "broken.csv", "negative.csv", "kavach.toml"]);
const first = guideProject("sales-first", []);
const main = "src/main.kav";

export const guideDataDoc: LessonDoc = {
  lead: "One task from start to finish: read sales from a CSV file into typed rows, total them per item, write a JSON report and read it back. Bad data is refused with the line and the column, and the tests find a bug before any user does.",
  simple: <>Spreadsheets save as CSV: rows of values with commas between. Here a program reads one, adds things up, and saves the answer as JSON, the format web services use. Kavach checks every value as it reads it.</>,
  minutes: 14,
  before: ["standard-library/tables/csv", "standard-library/json", "standard-library/files"],
  body: (
    <>
      <P>The data is a CSV file of sales, one per line, with a header:</P>
      <CodeBlock code={report.file("sales.csv")} file="sales.csv" lang="text" />
      <H2>1. A row is a type</H2>
      <P>
        Each column is a field, and each field&apos;s type says what the column may hold. Money is a{" "}
        <C>Decimal</C>, so 2.50 stays exactly 2.50. <C>units</C> is a whole number from 0 to a million (why the bound
        matters is in step 6):
      </P>
      <CodeBlock code={report.piece(main, "type Sale")} file={main} />
      <P>The report&apos;s rows are a type too:</P>
      <CodeBlock code={report.piece(main, "type Total")} file={main} />
      <H2>2. Work on the rows</H2>
      <P>
        Group the sales by item, add up each group, and sort the best seller first. Each step is a method you already
        know from <Link href="/docs/collections/grouping-and-statistics/">lists</Link>:
      </P>
      <CodeBlock code={report.piece(main, "fn totals")} file={main} />
      <H2>3. Read, write, read back</H2>
      <P>
        <C>table.read_csv&lt;Sale&gt;</C> reads the file as it goes and checks every cell against <C>Sale</C>.{" "}
        <C>json.stringify</C> writes the report, and <C>fs.write_text</C> replaces the file whole or not at all: a
        crash never leaves half a file. Reading it back with <C>json.decode&lt;[Total]&gt;</C> checks it the same way:
      </P>
      <CodeBlock code={report.piece(main, "fn main")} file={main} />
      <H2>4. Run it</H2>
      <Terminal session={report.said("check")} />
      <Terminal session={report.said("run src/main.kav")} />
      <H2>5. When the data is wrong</H2>
      <P>A row with a word where a number goes:</P>
      <CodeBlock code={report.file("broken.csv")} file="broken.csv" lang="text" />
      <Terminal session={report.said("run src/main.kav broken.csv")} />
      <P>A number outside the bound the type gives:</P>
      <CodeBlock code={report.file("negative.csv")} file="negative.csv" lang="text" />
      <Terminal session={report.said("run src/main.kav negative.csv")} />
      <P>
        Each error names the line, the column, what was expected and what was found, so the person who made the file
        can fix it. Nothing was half read: the program stops before it writes any report.
      </P>
      <H2>6. Let the tests find what you missed</H2>
      <P>
        The first version of this program had <C>units: Int</C>, any whole number, and summed the units as they
        were. Its example passed. Then <C>kavach test</C> tried 200 generated lists of sales, and found two whose units
        add up to more than an <C>Int</C> holds, shrunk to the smallest such case:
      </P>
      <Terminal session={first.said("test --seed 7")} />
      <P>
        The case is two sales of a negative number of units, huge ones. No real sale is like that, so the fix is to say
        so in the type: <C>units: Int&lt;range: 0..=1_000_000&gt;</C>. Generated sales then stay in that range, and
        the same line refuses a negative count in the CSV (step 5).
        A total can be larger than one sale, so it is summed as a plain <C>Int</C>:{" "}
        <C>p.1.sum(s =&gt; Int(s.units))</C>. Now every generated case passes:
      </P>
      <Terminal session={report.said("test --seed 7")} />
      <Note>
        The failing case was saved under <C>tests/found/</C> and is tried on every run after, so the bug can&apos;t come
        back unnoticed. See <Link href="/docs/errors/contracts/">Contracts</Link>.
      </Note>
      <H2>The whole program</H2>
      <CodeBlock code={report.file(main)} file={main} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/34-table.md" label="spec 14.34 · std.table" />
        <SrcRef file="docs/spec/02-types/12-json.md" label="spec 2.12 · JSON" />
        <SrcRef file="docs/spec/14-standard-library/03-fs.md" label="spec 14.3 · write_text is atomic" />
        <SrcRef file="docs/spec/11-testing.md" label="spec 11.3 · generated values" />
        <SrcRef file="website/content/samples/guides/sales-report.project/runs" label="the commands this page runs" />
      </div>
    </>
  ),
};
