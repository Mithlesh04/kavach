import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Time › Patterns ----------
//
// Spec 14.4 *Patterns*: d.format(pattern) writes a Date, Time or DateTime
// as a pattern says, and parse_date/parse_time/parse_datetime read what it
// writes. The tables are the spec's own.

const TESTS = "tests/conformance/14-standard-library";

export const timePatternsDoc: LessonDoc = {
  lead: "A pattern writes a date or a time the way people or other systems want it, 4 Nov 2026 or an HTTP date, and reads the same text back. Its letters are fields; everything else is written as it is.",
  simple: <>A pattern is a stencil for dates: <C>D MMM YYYY</C> means &quot;day, short month name, year&quot;, so the date comes out as <C>4 Nov 2026</C>. The same stencil reads that text back into a date.</>,
  minutes: 5,
  before: ["standard-library/time"],
  body: (
    <>
      <Ex name="time/patterns" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Letters from the table below are fields. Spaces, commas, <C>/</C> and <C>:</C> are written as they are.</li>
        <li>Text in single quotes is written as it is, letters and all: <C>&apos;GMT&apos;</C>, <C>&apos;at&apos;</C>. Two quotes, <C>&apos;&apos;</C>, write one.</li>
        <li>The third line is an HTTP date: a moment in UTC, through <C>time.utc</C>.</li>
      </ul>
      <H2>The fields</H2>
      <ModuleTable file="04-time.md" number="14.4" first="Field" heading="Patterns" />
      <H2>Reading text back</H2>
      <ModuleTable file="04-time.md" number="14.4" first="Method or function" heading="Patterns" />
      <P>
        Reading takes only what <C>format</C> writes with the same pattern. Anything else fails with{" "}
        <C>Invalid(text)</C>: a name in other letters, a two-digit year, a weekday that isn&apos;t the date&apos;s, or a
        date that doesn&apos;t exist:
      </P>
      <Ex name="time/patterns-read" />
      <H2>Mistakes it keeps out</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Names are always English.</strong> The text never depends on the machine&apos;s language or region, so every machine writes the same text and reads it back.</li>
        <li><strong className="text-ink">Only the table&apos;s letters are fields.</strong> <C>yyyy</C>, <C>dd</C> and <C>%Y</C> are mistakes, not something else written quietly. There is no two-digit year.</li>
        <li><strong className="text-ink">Minutes and months can&apos;t be swapped.</strong> <C>mm</C> (minutes) needs an hour in the pattern, and <C>M</C> or <C>MM</C> (the month) a day or a year.</li>
        <li><strong className="text-ink">A 12-hour clock says AM or PM.</strong> <C>h</C> and <C>hh</C> need <C>A</C>, and <C>A</C> needs one of them.</li>
        <li><strong className="text-ink">Each field appears once,</strong> and a <C>Date</C> takes only date fields, a <C>Time</C> only time fields.</li>
      </ul>
      <P>
        A pattern that breaks these rules is a mistake in the program, not in its data, so it breaks{" "}
        <C>format</C>&apos;s <Link href="/docs/errors/contracts/require/">require</Link>. Here <C>mm</C> means minutes,
        which a date doesn&apos;t have:
      </P>
      <Ex name="time/pattern-swapped.panics" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/04-time.md" label="spec 14.4 · patterns" />
        <SrcRef file={`${TESTS}/time-patterns-write-dates-and-times.kav`} label="writing, tested" />
        <SrcRef file={`${TESTS}/time-patterns-read-what-they-write.kav`} label="reading, tested" />
        <SrcRef file={`${TESTS}/time-patterns-refuse-swapped-minutes-and-months.kav`} label="swapped fields, tested" />
        <SrcRef file={`${TESTS}/time-patterns-twelve-hours-need-am-or-pm.kav`} label="AM and PM, tested" />
      </div>
    </>
  ),
};
