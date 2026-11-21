import Link from "next/link";
import { Explainer } from "@/components/explain/explainer";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- Lists › Swapping items ----------
//
// Spec 2.2, "Swapping two items"; tuple assignment is spec 1.14.

export const swapListDoc: LessonDoc = {
  lead: "xs.swap(i, j) swaps two items of a list, in place.",
  simple: <>Like two people in a queue changing places: nobody else moves, and nobody leaves the queue.</>,
  minutes: 2,
  before: ["collections/lists/changing-items"],
  body: (
    <>
      <Ex name="lists/swap" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>swap</C> gives nothing back: it changes the list.</li>
        <li><C>xs.swap(i, i)</C> changes nothing.</li>
        <li>It works on an item or a dict entry too, as <C>push</C> does: <C>rows[1].swap(0, 1)</C> changes the list inside.</li>
        <li>When nothing else holds the list, the two items change places where they are. Nothing is copied, text and records included.</li>
      </ul>
      <P>An index out of range stops the program, as <C>xs[i]</C> does, and the list doesn&apos;t change:</P>
      <Ex name="lists/swap-range.panics" />
      <P>Swapping changes the list, so like <C>push</C> it needs a <C>let</C>:</P>
      <Ex name="lists/swap-const.fails" />
      <P>
        A function with two <C>inout</C> parameters can&apos;t take two items of one list: that would be two names for
        one value. The error says what to write instead:
      </P>
      <Ex name="functions/inout-swap-items.fails" />
      <P>
        To swap any two places, such as items of two lists or two fields, use{" "}
        <Link href="/docs/basics/variables/tuple-assignment/">tuple assignment</Link>:{" "}
        <C>(xs[i], ys[j]) = (ys[j], xs[i])</C>.
      </P>
      <H2>What the machine does</H2>
      <P>A list in memory, step by step; its seventh step is a swap:</P>
      <Explainer id="list-buffer" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/02-collections.md" label="spec 2.2 · swapping two items" />
        <SrcRef file="docs/spec/14-standard-library/02-built-in-methods.md" label="spec 14.2 · items.swap" />
        <SrcRef file="tests/conformance/02-types/swap-changes-two-items-in-place.kav" label="tested" />
        <SrcRef file="tests/conformance/02-types/swap-works-on-an-item-and-a-dict-entry.kav" label="an item or entry, tested" />
        <SrcRef file="tests/conformance/02-types/swapping-an-item-with-itself-changes-nothing.kav" label="with itself, tested" />
        <SrcRef file="tests/conformance/02-types/swap-out-of-range-panics.kav" label="out of range, tested" />
        <SrcRef file="tests/conformance/02-types/swap-needs-a-let.kav" label="needs a let, tested" />
      </div>
    </>
  ),
};
