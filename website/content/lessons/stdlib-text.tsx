import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Text and Collections ----------

export const textCollectionsDoc: LessonDoc = {
  lead: "std.text and std.collections hold small helpers for text and lists: padding, words, chunks, pairs, counts. They are written in Kavach itself.",
  simple: <>These are the small tools you would otherwise write again in every program: filling a number with zeros, splitting text into words, counting how often each item appears. They come ready, tested, and written in Kavach.</>,
  minutes: 3,
  before: ["basics/strings", "collections/lists"],
  body: (
    <>
      <H2>Text</H2>
      <Ex name="text/helpers" />
      <P>
        To find numbers inside text, see{" "}
        <Link href="/docs/basics/strings/strings-to-numbers/">Strings to numbers</Link>.
      </P>
      <H2>Text that looks the same</H2>
      <P>
        One letter can be written with different code points: <C>é</C> as one, or as <C>e</C> and an accent that
        joins it. Text typed on one machine and pasted from another can look the same and still not be equal. A
        Unicode normal form writes it one way, so both compare equal:
      </P>
      <Ex name="text/normal-forms" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>len</C> counts code points, so the two spellings of <C>café</C> have 4 and 5.</li>
        <li><C>nfc</C> joins a letter and its accent; <C>nfd</C> splits them.</li>
        <li><C>nfkc</C> and <C>nfkd</C> also turn look-alike characters into plain ones: <C>ﬁ</C> into <C>fi</C>, the full-width <C>１２</C> into <C>12</C>.</li>
        <li>To compare what people type, such as user names, passwords and searches, normalise both sides first.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/14-standard-library/text-normal-forms.kav" label="normal forms, tested" />
        <SrcRef file="docs/decisions/0102-unicode-normalization.md" label="decision 0102" />
      </div>
      <H2>Lists</H2>
      <Ex name="collections/helpers" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>zip</C> stops at the shorter list, so <C>&quot;c&quot;</C> has no pair.</li>
        <li><C>unique</C> and <C>counts</C> keep the order in which items were first seen.</li>
      </ul>
      <H2>Every function</H2>
      <ModuleTable file="09-text-and-collections.md" number="14.9" first="Function" />
      <P>
        Both modules are written in Kavach, with contracts and examples like any other code, so <C>kavach test</C>{" "}
        checks them too. The four normal forms are the exception: the runtime gives them, from Unicode&apos;s own
        tables.
      </P>
      <SrcRef file="docs/spec/14-standard-library/09-text-and-collections.md" label="spec 14.9 · std.text and std.collections" />
    </>
  ),
};
