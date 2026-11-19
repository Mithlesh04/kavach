import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { fileTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Collections with a limit, lists of fixed length, and rings ----------
//
// Spec 2.2 (decision 0137): [T]<max: n>, {K: V}<max: n> and Set<T, max: n>
// hold at most n entries; [T]<len: n> holds exactly n; Ring<T, max: n>
// keeps the last n items pushed. capacity(n) is for an n known only while
// running.

const SPEC = "docs/spec/02-types/02-collections.md";
const TESTS = "tests/conformance/02-types";

/** The spec's table of the ways a limit is written. */
const written = () => {
  const t = fileTables(SPEC).find((x) => x.header[0] === "Written" && x.header[1] === "Holds");
  if (!t) throw new Error(`${SPEC}: no table of limits`);
  return t;
};

export const capacityDoc: LessonDoc = {
  lead: "A list, dict or set may say in its type how many entries it holds at most: [Int]<max: 3>. Room for all of them is made at once, and adding to a full one is a panic.",
  simple: <>Like a parking lot with 3 spaces: the spaces are there from the start, and a fourth car can&apos;t get in. A program that knows how many things it keeps says so in the type, and the collection never grows past it.</>,
  minutes: 8,
  before: ["collections/lists", "collections/dicts", "collections/sets"],
  body: (
    <>
      <Ex name="limits/max" />
      <P>
        The limit goes in <C>&lt;…&gt;</C> right after the collection&apos;s type, with its name, <C>max</C>. Each
        collection has two spellings, the short one and the one with the type&apos;s name:
      </P>
      <Table header={written().header} rows={written().rows} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Room for all the entries is made at once, so filling it never stops to make room.</li>
        <li><C>is_full()</C> says whether it holds as many entries as its limit.</li>
        <li>Changing a key that is already in a dict, or adding a value a set already has, adds nothing, so it never panics.</li>
        <li>A limit is from 0 to 4,294,967,294 (2³² − 2); another is a compile error.</li>
        <li>
          <C>&lt;…&gt;</C> belongs to the type just before it: <C>{"{String: [Int]<max: 5>}"}</C> is a dict of lists
          of at most 5 items each, with no limit on its keys.
        </li>
      </ul>
      <P>The item types may be left out. They are worked out from what the collection is given, as for <C>let a = []</C>:</P>
      <Ex name="limits/infer" />
      <P>A literal with more entries than the limit is a compile error:</P>
      <Ex name="limits/literal-too-long.fails" />
      <P>The limit is written after the brackets, not inside them, where it would read as a list of two items:</P>
      <Ex name="limits/inside.fails" />
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
        The limit goes with the collection. A copy has it too (<C>let b = a</C>), and so does the collection after{" "}
        <C>push</C>, <C>+=</C>, <C>add</C>, <C>remove</C>, <C>pop</C> or an item set in place. A new collection made
        from it, as by <C>xs + ys</C>, <C>sort()</C>, <C>map</C>, <C>filter</C> or <C>keys()</C>, has no limit, and
        its type is the plain one:
      </P>
      <Ex name="limits/kept" />
      <P>So a copy that is full panics too, even when the value it was copied from isn&apos;t full:</P>
      <Ex name="limits/copy-full.panics" />
      <P>A collection with a limit may be given where a plain one is expected, and it keeps its limit there:</P>
      <Ex name="limits/plain-param" />
      <H2>Making a plain collection limited</H2>
      <P>
        The other way, a plain collection into one with a limit, may not fit, so it takes <C>try</C> or{" "}
        <C>catch</C>. The type with its limit, called with the collection, fails with a <C>SizeError</C> when it has
        too many entries, as <C>try Int(text)</C> fails when the text is no number:
      </P>
      <Ex name="limits/try-max" />
      <P>Without <C>try</C> or <C>catch</C> it is a compile error, unless the collection is a literal, which is checked while compiling:</P>
      <Ex name="limits/no-try.fails" />
      <H2>A limit known only while running</H2>
      <P>
        When the limit is worked out while the program runs, <C>List.capacity(n)</C>, <C>Dict.capacity(n)</C> and{" "}
        <C>Set.capacity(n)</C> make an empty collection of at most <C>n</C> entries. Its type is the plain one, and
        the limit belongs to the value. A second argument gives the first entries:
      </P>
      <Ex name="limits/runtime" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A number below 0 counts as 0. One above 4,294,967,294, or more first entries than <C>n</C>, is a panic.</li>
        <li>With a number known while compiling, <C>capacity</C> is a compile error that names the type to write:</li>
      </ul>
      <Ex name="limits/known-capacity.fails" />
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
      <H2>A list of fixed length</H2>
      <P>
        <C>[Float]&lt;len: 3&gt;</C> holds exactly 3 items, always: a point in space, a colour, a row of a grid. An
        item may be set in place. <C>List&lt;T, len: n&gt;.filled(v)</C> makes one of <C>n</C> items, each{" "}
        <C>v</C>; it never fails, so it needs no <C>try</C>:
      </P>
      <Ex name="limits/fixed" />
      <P><C>push</C>, <C>pop</C>, <C>remove</C> and <C>+=</C> would change how many items it holds, so they are compile errors:</P>
      <Ex name="limits/fixed-push.fails" />
      <P>An index written as a number is checked while compiling:</P>
      <Ex name="limits/fixed-index.fails" />
      <P>
        Given where a plain list is expected, it is a plain list there: its length doesn&apos;t go with it, so the code
        that takes it may change how many items it holds. A <C>max</C> limit, above, does go with the value:
      </P>
      <Ex name="limits/fixed-plain" />
      <Note>
        A function can take the length as a number parameter, as <C>fn dot&lt;n: Int&gt;(a: [Float]&lt;len: n&gt;, …)</C>{" "}
        does. See <Link href="/docs/types/generics/">Generics</Link>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.2 · collections" />
        <SrcRef file="docs/decisions/0137-numbers-in-types.md" label="decision 0137" />
        <SrcRef file="compiler/checker/src/sizes.rs" label="the checker's limits" />
        <SrcRef file={`${TESTS}/a-limit-in-the-type-holds-at-most-max.kav`} label="tested" />
        <SrcRef file={`${TESTS}/a-push-past-a-limit-in-the-type-panics.kav`} label="a full list, tested" />
        <SrcRef file={`${TESTS}/capacity-copy-keeps-limit-panics.kav`} label="copies, tested" />
        <SrcRef file={`${TESTS}/try-into-a-limit-fails-with-a-size-error.kav`} label="try, tested" />
        <SrcRef file={`${TESTS}/capacity-with-a-known-number-is-an-error.kav`} label="capacity, tested" />
        <SrcRef file={`${TESTS}/capacity-mixed-and-nested.kav`} label="mixed and nested, tested" />
        <SrcRef file={`${TESTS}/a-fixed-length-list-is-filled.kav`} label="fixed length, tested" />
        <SrcRef file={`${TESTS}/a-fixed-length-list-has-no-push.kav`} label="no push, tested" />
      </div>
    </>
  ),
};

export const ringDoc: LessonDoc = {
  lead: "A Ring<T, max: n> keeps the last n items pushed. When it is full, push drops the oldest item to make room.",
  simple: <>Like a phone&apos;s list of recent calls: it shows the last few, and each new call pushes the oldest one off the end. Good for the last few readings, messages or frame times.</>,
  minutes: 3,
  before: ["collections/collections-with-a-limit"],
  body: (
    <>
      <Ex name="limits/ring" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <C>Ring()</C> makes an empty ring when the variable&apos;s type says its size;{" "}
          <C>Ring&lt;T, max: n&gt;()</C> says it where it is made.
        </li>
        <li>A list given, as in <C>Ring&lt;T, max: n&gt;(items)</C>, makes the first items; given more than <C>n</C>, it keeps the last <C>n</C>. So making a ring never fails.</li>
        <li>The items are in the order they were pushed, oldest first: <C>first()</C> and <C>get(0)</C> are the oldest, <C>last()</C> the newest, and <C>for</C> visits them in that order.</li>
        <li>It has <C>push</C>, <C>len</C>, <C>is_empty</C>, <C>is_full</C>, <C>first</C>, <C>last</C>, <C>get</C>, <C>contains</C> and <C>to_list()</C>, which gives a list of its items with no limit.</li>
        <li>It prints as that list. A copy of a ring keeps its size.</li>
      </ul>
      <P>
        When the size is worked out while the program runs, <C>Ring.capacity(n)</C> makes a <C>Ring&lt;T&gt;</C> of
        that size, as <C>List.capacity(n)</C> does for a list:
      </P>
      <Ex name="limits/ring-runtime" />
      <H2>A ring is not a list</H2>
      <P>
        A list with a limit panics when it is full, and a ring drops an item. So a ring is a type of its own: a
        function that takes a list is never given a ring, and pushing onto a list never drops an item without saying
        so. Use <C>to_list()</C> to pass its items on:
      </P>
      <Ex name="limits/ring-not-a-list.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.2 · rings" />
        <SrcRef file={`${TESTS}/a-ring-in-its-type-keeps-the-last-n.kav`} label="tested" />
        <SrcRef file={`${TESTS}/ring-keeps-the-last-n.kav`} label="Ring.capacity, tested" />
        <SrcRef file={`${TESTS}/ring-is-not-a-list.kav`} label="not a list, tested" />
        <SrcRef file={`${TESTS}/ring-holds-records.kav`} label="records, tested" />
      </div>
    </>
  ),
};
