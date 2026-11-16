import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- Collections with a limit, and rings ----------
//
// Spec 2.2: List/Map/Set.capacity(n) hold at most n entries; Ring<T>
// keeps the last n items pushed.

const TESTS = "tests/conformance/02-types";

export const capacityDoc: LessonDoc = {
  lead: "List.capacity(n), Map.capacity(n) and Set.capacity(n) make a collection that holds at most n entries. Room for all n is made at once, and adding to a full one is a panic.",
  simple: <>Like a parking lot with 3 spaces: the spaces are there from the start, and a fourth car can&apos;t get in. A program that knows how many things it keeps says so, and the collection never grows past it.</>,
  minutes: 5,
  before: ["collections/lists", "collections/maps", "collections/sets"],
  body: (
    <>
      <Ex name="limits/capacity" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The first argument is the limit. Room for all of it is made at once, so filling it never stops to make room.</li>
        <li>The second argument, if given, is the first entries: a list for <C>List.capacity</C> and <C>Set.capacity</C>, a map for <C>Map.capacity</C>.</li>
        <li><C>is_full()</C> says whether it holds as many entries as its limit.</li>
        <li>Changing a key that is already in a map, or adding a value a set already has, adds nothing, so it never panics.</li>
        <li>A limit below 0 counts as 0. One above 4,294,967,294 (2³² − 2), or more first entries than the limit, is a panic.</li>
      </ul>
      <H2>Adding to a full one</H2>
      <P>
        A full collection never grows. Adding one more entry is a{" "}
        <Link href="/docs/errors/panics/">panic</Link> that says how big it is:
      </P>
      <Ex name="limits/full.panics" />
      <P>When being full is normal, check with <C>is_full()</C> first:</P>
      <Ex name="limits/check-first" />
      <P>
        To keep the newest items and drop the oldest instead, use a{" "}
        <Link href="/docs/collections/rings/">ring</Link>.
      </P>
      <H2>Which values keep the limit</H2>
      <P>
        The limit belongs to the value. A copy has it too (<C>let b = a</C>), and so does the collection after{" "}
        <C>push</C>, <C>+=</C>, <C>add</C>, <C>remove</C>, <C>pop</C> or an item set in place. A new collection made
        from it, as by <C>xs + ys</C>, <C>sort()</C>, <C>map</C>, <C>filter</C> or <C>keys()</C>, has no limit:
      </P>
      <Ex name="limits/kept" />
      <P>So a copy that is full panics too, even when the value it was copied from isn&apos;t full:</P>
      <Ex name="limits/copy-full.panics" />
      <H2>Any kind of item</H2>
      <P>
        The items can be of any type. Items of different kinds go in one collection through a{" "}
        <Link href="/docs/types/unions/">union</Link>, and an inner collection with a limit keeps its own. More
        shapes are in <Link href="/docs/collections/nested-and-mixed-collections/">Nested and Mixed Collections</Link>:
      </P>
      <Ex name="limits/mixed" />
      <Note>
        Tuples need no limit: their size is fixed by their type. See{" "}
        <Link href="/docs/collections/tuples/">Tuples</Link>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/02-collections.md" label="spec 2.2 · collections" />
        <SrcRef file={`${TESTS}/capacity-holds-at-most-n.kav`} label="tested" />
        <SrcRef file={`${TESTS}/capacity-full-list-panics.kav`} label="a full list, tested" />
        <SrcRef file={`${TESTS}/capacity-copy-keeps-limit-panics.kav`} label="copies, tested" />
        <SrcRef file={`${TESTS}/capacity-mixed-and-nested.kav`} label="mixed and nested, tested" />
      </div>
    </>
  ),
};

export const ringDoc: LessonDoc = {
  lead: "A Ring<T> keeps the last n items pushed. When it is full, push drops the oldest item to make room.",
  simple: <>Like a phone&apos;s list of recent calls: it shows the last few, and each new call pushes the oldest one off the end. Good for the last few readings, messages or frame times.</>,
  minutes: 3,
  before: ["collections/collections-with-a-limit"],
  body: (
    <>
      <Ex name="limits/ring" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Ring.capacity(n)</C> makes an empty ring. A second argument gives the first items; given more than <C>n</C>, it keeps the last <C>n</C>.</li>
        <li>The items are in the order they were pushed, oldest first: <C>first()</C> and <C>get(0)</C> are the oldest, <C>last()</C> the newest, and <C>for</C> visits them in that order.</li>
        <li>It has <C>push</C>, <C>len</C>, <C>is_empty</C>, <C>is_full</C>, <C>first</C>, <C>last</C>, <C>get</C>, <C>contains</C> and <C>to_list()</C>, which gives a list of its items with no limit.</li>
        <li>It prints as that list. A copy of a ring keeps its size.</li>
      </ul>
      <H2>A ring is not a list</H2>
      <P>
        A list with a limit panics when it is full, and a ring drops an item. So <C>Ring&lt;T&gt;</C> is a type of its
        own: a function that takes a list is never given a ring, and pushing onto a list never drops an item without
        saying so. Use <C>to_list()</C> to pass its items on:
      </P>
      <Ex name="limits/ring-not-a-list.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/02-collections.md" label="spec 2.2 · rings" />
        <SrcRef file={`${TESTS}/ring-keeps-the-last-n.kav`} label="tested" />
        <SrcRef file={`${TESTS}/ring-is-not-a-list.kav`} label="not a list, tested" />
        <SrcRef file={`${TESTS}/ring-holds-records.kav`} label="records, tested" />
      </div>
    </>
  ),
};
