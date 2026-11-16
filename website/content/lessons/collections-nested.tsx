import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- Nested and mixed collections ----------
//
// Spec 2.2 says items can be of any type, and items of different kinds go
// in one collection through a union (2.6) or as Json (2.12). Every shape
// below is from a sample that ran.

const shapes: [string, string, string][] = [
  ["[[Int]]", "a list of lists", "[[1, 2], [3]]"],
  ["[String: [String]]", "a map of lists", "[\"fruit\": [\"apple\"]]"],
  ["[[String: String]]", "a list of maps", "[[\"name\": \"Asha\"]]"],
  ["[String: [String: [Int]]]", "a map of maps of lists", "[\"pens\": [\"blue\": [10]]]"],
  ["[String: Set<String>]", "a map of sets", "[\"pen\": Set.of(\"blue\")]"],
  ["[(Int, Int)]", "a list of tuples", "[(1, 2), (3, 4)]"],
  ["[(Int, Int): String]", "a map with tuple keys", "[(0, 0): \"start\"]"],
  ["Set<(Int, Int)>", "a set of tuples", "Set.of((1, 2))"],
  ["([String], [String: Int])", "a tuple of a list and a map", "([\"a\"], [\"a\": 1])"],
  ["[Cell]", "a list of a union: items of different kinds", "[Cell.Empty, Cell.Text(value: \"pens\")]"],
];

export const nestedDoc: LessonDoc = {
  lead: "Collections can hold collections: lists of lists, maps of lists, lists of maps, sets and tuples inside any of them, at any depth. Items of different types go in one collection through a union, or as Json.",
  simple: <>Like boxes inside boxes: a shelf of trays, each tray holding pens. Each box holds one kind of thing, and a box can hold other boxes. To keep different kinds in one box, give them one shared label first: a union.</>,
  minutes: 8,
  before: ["collections/lists", "collections/maps", "collections/sets", "collections/tuples"],
  body: (
    <>
      <P>A list of lists is a grid. Read an item with two indexes, and change one in place:</P>
      <Ex name="nested/grid" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>grid[1][2]</C> is row 1, then item 2 of that row. Both count from 0.</li>
        <li><C>grid[1][2] = 60</C> and <C>grid[0].push(4)</C> change an inner list, so the outer one needs a <C>let</C>.</li>
        <li>The inner lists can have different lengths.</li>
      </ul>
      <H2>Writing the type</H2>
      <P>
        Write the inner type inside the outer one. Kavach usually works out the type from the literal, so you write it
        when the collection starts empty, or to make it clear:
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Type</th>
              <th className="py-2 pr-4 font-bold">What it is</th>
              <th className="py-2 font-bold">A value</th>
            </tr>
          </thead>
          <tbody>
            {shapes.map(([t, what, v]) => (
              <tr key={t} className="border-b border-line">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{t}</C></td>
                <td className="py-2 pr-4 text-muted">{what}</td>
                <td className="py-2"><C>{v}</C></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>Maps of lists</H2>
      <P>
        A map gives an <Link href="/docs/types/optionals/">optional</Link>, as the key may be missing, so{" "}
        <C>?? []</C> gives an empty list then. To change an inner list, take it out, change it and put it back:
      </P>
      <Ex name="nested/map-of-lists" />
      <P>Calling <C>push</C> straight on the lookup is refused, as it may be <C>none</C>:</P>
      <Ex name="nested/inner-push.fails" />
      <H2>Lists of maps</H2>
      <Ex name="nested/list-of-maps" />
      <Note>
        When every map has the same keys, as here, a <Link href="/docs/types/records/">record</Link> is better:{" "}
        <C>Person(name: &quot;Asha&quot;, city: &quot;Pune&quot;)</C>. Its fields are checked when the program is
        built, and reading one never gives an optional.
      </Note>
      <H2>Deeper</H2>
      <P>
        Go down one level at a time. Each <C>??</C> gives a default when a key is missing, so a missing key anywhere
        gives the default, not a crash:
      </P>
      <Ex name="nested/deep" />
      <H2>Tuples inside and around</H2>
      <P>
        Tuples go in lists, in sets and as map keys, which suits points on a grid. A tuple can hold collections too,
        and <C>const (a, b) = …</C> takes it apart:
      </P>
      <Ex name="nested/tuples" />
      <H2>Sets inside</H2>
      <Ex name="nested/sets" />
      <H2>Copies are whole</H2>
      <P>
        A collection is a value. A copy never shares changes with the one it came from, however deep the change. See{" "}
        <Link href="/docs/memory/values-not-references/">Values, not References</Link>.
      </P>
      <Ex name="nested/copies" />
      <H2>Items of different types</H2>
      <P>All the items of one collection have one type. Mixing types is refused:</P>
      <Ex name="nested/mixed.fails" />
      <P>
        To keep different kinds of items together, declare a <Link href="/docs/types/unions/">union</Link> with one
        variant for each kind. <C>match</C> then tells them apart, and must cover every kind:
      </P>
      <Ex name="nested/mixed-union" />
      <P>
        For data whose shape is only known when the program runs, such as a web service&apos;s answer, use{" "}
        <C>Json</C>. Where a <C>Json</C> is expected, a literal can mix numbers, text, <C>true</C>,{" "}
        <C>none</C>, lists and maps at any depth. See{" "}
        <Link href="/docs/standard-library/json/json-values/">JSON values</Link>.
      </P>
      <Ex name="nested/mixed-json" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/02-collections.md" label="spec 2.2 · collections" />
        <SrcRef file="docs/spec/02-types/03-tuples.md" label="spec 2.3 · tuples" />
        <SrcRef file="docs/spec/02-types/06-unions.md" label="spec 2.6 · unions" />
        <SrcRef file="docs/spec/02-types/12-json.md" label="spec 2.12 · Json" />
        <SrcRef file="tests/conformance/02-types/capacity-mixed-and-nested.kav" label="mixed and nested, tested" />
      </div>
    </>
  ),
};
