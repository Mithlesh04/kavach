import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Grouping and statistics ----------
//
// Spec 14.2's "Grouping and statistics" (decision 0135): a list of records
// works like a table. Both tables come from the spec itself.

const FILE = "14-standard-library/02-built-in-methods.md";
const TESTS = "tests/conformance/14-standard-library";

/** The spec's tables under "Grouping and statistics": the list methods, then the dict ones. */
const tables = () => {
  const found = specTables(FILE, "14.2").filter((t) => t.heading === "Grouping and statistics" && t.header[0] === "Method");
  if (found.length !== 2) throw new Error(`spec 14.2: expected the list and dict tables of grouping, found ${found.length}`);
  return found;
};

export const groupingDoc: LessonDoc = {
  lead: "A list of records works like a table: group_by, count_by and unique split it by a key, mean, median, std and describe sum it up, and inner_join and left_join join two lists on a key.",
  simple: <>A list of sales is like a spreadsheet. You can ask how many sales each region had, what the average sale was, or put each order next to the customer who made it, without a loop.</>,
  minutes: 8,
  before: ["collections/lists", "collections/dicts", "types/records"],
  body: (
    <>
      <Ex name="grouping/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          Every key and every column is a lambda, so the compiler checks it: <C>.region</C> is short for{" "}
          <C>s =&gt; s.region</C>. A misspelled field is a compile error, not a wrong answer.
        </li>
        <li>Picking columns out is <C>map</C> to an anonymous record; there is no <C>select</C>.</li>
        <li>The same methods work as pandas, polars and R&apos;s dplyr have them, with one name for each job.</li>
      </ul>
      <Ex name="grouping/misspelled.fails" />
      <H2>Grouping and counting</H2>
      <Ex name="grouping/group" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>group_by</C> gives a dict of lists: the keys in the order first seen, each group&apos;s items in their order.</li>
        <li><C>map_values</C> turns each group into one value, keeping the keys. A sum per key is <C>group_by(key).map_values(g =&gt; g.sum(f))</C>.</li>
        <li><C>to_list()</C> gives the dict&apos;s pairs as a list, so the list&apos;s methods apply: sorting by count, taking the top two.</li>
        <li>A key is any value, compared as a dict compares its keys. A tuple groups by several keys at once.</li>
        <li><C>unique()</C> keeps each item once, and <C>unique_by(key)</C> the first item with each key.</li>
      </ul>
      <H2>Statistics</H2>
      <Ex name="grouping/stats" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>mean</C> and <C>median</C> give a <C>Float</C> for whole numbers and <C>Float</C>, a <C>Float32</C> for <C>Float32</C>, and a <C>Decimal</C> for <C>Decimal</C>. <C>var</C>, <C>std</C> and <C>describe</C> give <C>Float</C>s.</li>
        <li>An empty list gives <C>none</C>, so each result is an optional, read with <C>??</C>.</li>
        <li><C>var</C> and <C>std</C> divide by the count minus 1, the sample form pandas and R use; <C>correction: 0</C> is NumPy&apos;s.</li>
        <li><C>mean</C> adds exactly and divides once, so it gives the same bits in any order, on any number of cores.</li>
        <li><C>describe</C> gives the count, mean, standard deviation, smallest, quartiles and largest. The quartiles are at 0.25, 0.5 and 0.75 of the way through the sorted values, as pandas, NumPy and R do.</li>
        <li><C>min_by</C> and <C>max_by</C> give the item, the first one on a tie.</li>
      </ul>
      <P>The mean of whole numbers is rarely whole, so it is never an <C>Int</C>:</P>
      <Ex name="grouping/mean-int.fails" />
      <P>Nothing is skipped without saying. A <C>NaN</C> makes the mean <C>NaN</C>, and sorts after every number; leave values out with <C>filter</C> first:</P>
      <Ex name="grouping/nan" />
      <P><C>min_by</C>&apos;s key needs an order, as <C>sort_by</C>&apos;s does:</P>
      <Ex name="grouping/min-by-record.fails" />
      <H2>Joining two lists</H2>
      <P>
        <C>inner_join(other, key, other_key, row)</C> gives <C>row(a, b)</C> for each pair whose keys are equal.{" "}
        <C>left_join</C> also gives <C>row(a, none)</C> for an item that matches nothing:
      </P>
      <Ex name="grouping/join" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The rows come in the order of the items; an item&apos;s matches in the order of <C>other</C>. Two matches give two rows, as SQL does.</li>
        <li>A join builds a dict of <C>other</C> by its key, so its time grows with the two lengths, not their product.</li>
        <li>A right join is a left join written the other way round.</li>
      </ul>
      <P>The two keys are of one type:</P>
      <Ex name="grouping/join-types.fails" />
      <H2>Pivots</H2>
      <P>
        <C>pivot(row_key, column_key, cell)</C> groups by two keys: for each row key, and in it each column key,{" "}
        <C>cell</C> of the items with both. A cell with no items isn&apos;t there, so it is read with <C>??</C>:
      </P>
      <Ex name="grouping/pivot" />
      <H2>One name for each job</H2>
      <P>Names other tools use are compile errors that name Kavach&apos;s:</P>
      <Ex name="grouping/value-counts.fails" />
      <Ex name="grouping/sum-by.fails" />
      <P>
        <C>math.mean(xs)</C>, <C>collections.unique</C> and <C>collections.counts</C> are now these list methods:{" "}
        <C>xs.mean(x =&gt; x)</C>, <C>items.unique()</C> and <C>items.count_by(x =&gt; x)</C>:
      </P>
      <Ex name="grouping/math-mean.fails" />
      <H2>Every method</H2>
      <P>Lists:</P>
      <Table header={tables()[0].header} rows={tables()[0].rows} wrapFirst />
      <P>Dicts:</P>
      <Table header={tables()[1].header} rows={tables()[1].rows} wrapFirst />
      <P>
        A range has these too, as it has every method that reads a list. For a big table kept in columns,{" "}
        <C>std.table</C> is planned with the same methods; for numbers in many dimensions, see{" "}
        <Link href="/docs/standard-library/tensors/reductions/">Tensors: Reductions</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.2 · grouping and statistics" />
        <SrcRef file="docs/decisions/0135-tables-and-grouping.md" label="decision 0135" />
        <SrcRef file="stdlib/collections/grouping.kav" line={14} symbol="fn group_by" />
        <SrcRef file="stdlib/collections/grouping.kav" line={76} symbol="fn inner_join" />
        <SrcRef file="stdlib/collections/stats.kav" line={19} symbol="fn mean" />
        <SrcRef file="stdlib/collections/stats.kav" line={77} symbol="fn describe" />
        <SrcRef file={`${TESTS}/group-by-keeps-the-order-first-seen.kav`} label="group_by, tested" />
        <SrcRef file={`${TESTS}/count-by-counts-each-key.kav`} label="count_by, tested" />
        <SrcRef file={`${TESTS}/unique-keeps-the-first-of-each.kav`} label="unique, tested" />
        <SrcRef file={`${TESTS}/mean-adds-exactly.kav`} label="mean, tested" />
        <SrcRef file={`${TESTS}/describe-gives-a-summary.kav`} label="describe, tested" />
        <SrcRef file={`${TESTS}/statistics-never-skip-nan.kav`} label="NaN, tested" />
        <SrcRef file={`${TESTS}/inner-join-gives-every-matching-pair.kav`} label="inner_join, tested" />
        <SrcRef file={`${TESTS}/left-join-keeps-items-that-match-nothing.kav`} label="left_join, tested" />
        <SrcRef file={`${TESTS}/pivot-groups-by-two-keys.kav`} label="pivot, tested" />
      </div>
    </>
  ),
};
