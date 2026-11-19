import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { WhatIsTested } from "./tested-table";

// ---------- Indexing and slicing ----------
//
// Spec 2.19, decision 0134: xs[i], xs[start:end:step], #k from the end,
// setting a slice, and the slice-like methods that are gone.

const TESTS = "tests/conformance/02-types";

export const slicingDoc: LessonDoc = {
  lead: "xs[i] reads one item. xs[start:end:step] is a slice: a new list with the items from start up to, not including, end. #k counts from the end.",
  simple: <>Think of a row of numbered seats. <C>xs[2]</C> is the person in seat 2. <C>xs[1:3]</C> is everyone from seat 1 up to seat 3, not counting seat 3. <C>#1</C> means the last seat, however long the row is.</>,
  minutes: 8,
  before: ["collections/lists"],
  body: (
    <>
      <Ex name="slicing/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>queue[0]</C> is the first item: positions start at 0.</li>
        <li><C>queue[#1]</C> is the last item, <C>queue[#2]</C> the one before it.</li>
        <li>A slice <C>queue[start:end]</C> holds the items from <C>start</C> up to, not including, <C>end</C>. A missing <C>start</C> is the first item, and a missing <C>end</C> is past the last.</li>
        <li>A third part is the step: <C>queue[::2]</C> takes every second item, and <C>queue[::-1]</C> walks backwards.</li>
      </ul>
      <P>The same brackets work on text, <C>Bytes</C> and text views:</P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Value</th>
              <th className="py-2 pr-4 font-bold"><C>x[i]</C> gives</th>
              <th className="py-2 font-bold"><C>x[start:end:step]</C> gives</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["A list [T]", "T", "[T]"],
              ["String", "Char, counted in characters", "String"],
              ["Bytes", "UInt8", "Bytes"],
              ["StrView", "Char", "StrView: a view of the same string"],
              ["A dict {K: V}", "d[key]: V?", "A compile error: a dict has keys, not places"],
              ["Your own type", "Its index method", "Its slice method"],
            ].map(([value, one, slice]) => (
              <tr key={value} className="border-b border-line align-top">
                <td className="py-2 pr-4">{value}</td>
                <td className="py-2 pr-4 text-muted">{one}</td>
                <td className="py-2 text-muted">{slice}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Your own types get the brackets through interfaces: see{" "}
        <Link href="/docs/types/operators-for-your-types/">Operators for Your Types</Link>.
      </P>

      <H2>A slice never goes out of range</H2>
      <P>
        Reading one item past the end is a panic, but a slice is <em>clamped</em>: a part past the end counts as the
        end, and a part worked out below 0 counts as before the first item. So a slice gives what there is:
      </P>
      <Ex name="slicing/clamped" />

      <H2>Steps</H2>
      <P>
        A negative step walks backwards, as in Python: it starts at <C>start</C> and stops before it reaches{" "}
        <C>end</C>. With a negative step, a missing <C>start</C> is the last item, and a missing <C>end</C> is before
        the first:
      </P>
      <Ex name="slicing/step" />
      <P>A step of 0 would never move. Written in the code, it is a compile error:</P>
      <Ex name="slicing/step-zero.fails" />
      <P>Worked out while the program runs, it is a panic:</P>
      <Ex name="slicing/step-zero.panics" />

      <H2>From the end: #k</H2>
      <P>
        <C>#k</C> is the place <C>k</C> from the end: <C>len - k</C>. It works in an index and at either end of a
        slice:
      </P>
      <Ex name="slicing/from-end" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>#</C> covers the whole part after it, up to <C>:</C>, <C>,</C> or <C>]</C>: <C>xs[#n + 1]</C> is <C>xs[xs.len() - (n + 1)]</C>.</li>
        <li>As the start of a slice, <C>#0</C> is the end, so <C>xs[#n:]</C> with <C>n</C> of 0 is empty: the last 0 items.</li>
        <li><C>#0</C> as an index is out of range, as <C>xs[xs.len()]</C> is.</li>
        <li><C>#</C> is written only inside <C>[ ]</C>.</li>
      </ul>

      <H2>A plain number counts from the start</H2>
      <P>
        In Python, <C>xs[-1]</C> is the last item. Kavach counts from the end only with <C>#</C>, so a written negative
        number is a compile error that names the <C>#</C> form:
      </P>
      <Ex name="slicing/negative.fails" />
      <P>
        That way arithmetic never changes what an index means. In Python, <C>xs[i - 1]</C> at <C>i = 0</C> quietly
        reads the last item. In Kavach it is out of range, and <C>get</C> gives <C>none</C>:
      </P>
      <Ex name="slicing/before.panics" />

      <H2>Text</H2>
      <P>
        Text is indexed and cut by characters, as <C>len()</C> counts them, so a letter such as <C>é</C> is one place.{" "}
        <C>s[i]</C> is a <C>Char</C>, and a slice of text is text:
      </P>
      <Ex name="slicing/text" />
      <P>Reading a character past the end is a panic that says how long the text is:</P>
      <Ex name="slicing/text-past.panics" />
      <Note>
        A slice of a <C>String</C> is a copy. <C>s.view()[a:b]</C> is a view of those characters, with nothing copied:
        see <Link href="/docs/basics/strings/text-views-and-builders/">Text views and builders</Link>.
      </Note>

      <H2>A slice is a new value</H2>
      <P>Changing a slice never changes the list it came from, and changing the list never changes the slice:</P>
      <Ex name="slicing/new-value" />

      <H2>Setting a slice</H2>
      <P>
        <C>xs[a:b:step] = ys</C> replaces the items the slice names with the items of <C>ys</C>, in order. The list
        must be a <C>let</C>. <C>#</C> works where one item is set, too:
      </P>
      <Ex name="slicing/set" />
      <P>
        <C>ys</C> must have as many items as the slice, so setting a slice never makes a list longer or shorter:{" "}
        <C>push</C>, <C>pop</C>, <C>+=</C> and <C>remove</C> do that. Another length is a panic that names both:
      </P>
      <Ex name="slicing/set-length.panics" />
      <P>The slice is clamped first, then the lengths are compared. When nothing else holds the list, it is changed in place.</P>

      <H2>Lists of lists</H2>
      <P>A list of lists is read one level at a time: <C>rows[1][2]</C> is item 2 of row 1.</P>
      <Ex name="slicing/nested" />
      <P>A list has one dimension, so one pair of brackets takes one part:</P>
      <Ex name="slicing/nested.fails" />
      <P>
        A tensor has as many dimensions as its shape, and takes one part for each, separated by commas, as NumPy
        writes them: <C>t[1, 2]</C>, <C>t[:, #1]</C>. See{" "}
        <Link href="/docs/standard-library/tensors/indexing/">Tensors: Indexing</Link>.
      </P>
      <P>A dict has keys, not places, so it has no slices:</P>
      <Ex name="slicing/dict.fails" />

      <H2>No methods that slice</H2>
      <P>
        A slice is written only with brackets. The methods that did a slice&apos;s job are gone, and each one is a
        compile error that names the bracket form:
      </P>
      <Ex name="slicing/gone.fails" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Was</th>
              <th className="py-2 font-bold">Write</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["xs.slice(1, 3)", "xs[1:3]", "on lists, text and Bytes"],
              ["xs.take(n)", "xs[:n]", ""],
              ["xs.skip(n)", "xs[n:]", ""],
              ["xs.reverse()", "xs[::-1]", ""],
              ["s.truncate(n)", "s[:n]", ""],
              ["v.view(a, b)", "v[a:b]", "on a view: a view of the same string"],
              ["s.view(a, b)", "s.view()[a:b]", "on a String"],
            ].map(([was, now, where]) => (
              <tr key={was} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{was}</C></td>
                <td className="py-2"><C>{now}</C>{where ? <span className="text-muted">, {where}</span> : null}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>A channel&apos;s <C>take()</C> is not a slice, and stays.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/19-indexing-and-slicing.md" label="spec 2.19 · indexing and slicing" />
        <SrcRef file="docs/decisions/0134-slicing-and-indexing.md" label="decision 0134" />
        <SrcRef file="compiler/parser/src/index.rs" line={15} symbol="fn index_inside" />
        <SrcRef file="compiler/checker/src/slicing.rs" line={52} symbol="fn sliced" />
        <SrcRef file="runtime/core/src/slices.rs" line={87} symbol="fn bounds" />
        <SrcRef file={`${TESTS}/slices-take-every-step-th-item.kav`} label="slices, tested" />
      </div>
      <WhatIsTested number="2.19" />
    </>
  ),
  tryIt: {
    task: <>From <C>[3, 8, 1, 9, 4]</C>, print the last two items, then the list backwards.</>,
    answer: <P><C>print(xs[#2:])</C> gives <C>[9, 4]</C>, and <C>print(xs[::-1])</C> gives <C>[4, 9, 1, 8, 3]</C>.</P>,
  },
};
