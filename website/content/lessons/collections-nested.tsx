import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { loadSample } from "@/lib/samples";
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

// Each row is a checked sample; its result is read from the sample's
// .expect, so the table can't say what the compiler doesn't do.
const changeCases: [sample: string, setup: string, step: string][] = [
  ["nested/change-const-map.fails", "let a = [2, 4]; const b = [\"numbers\": a]", "b[\"numbers\"].push(6)"],
  ["nested/inner-push.fails", "a let map of lists", "groups[\"fruit\"].push(\"mango\")"],
  ["nested/change-let-put-back", "let b = [\"numbers\": a]", "take out, push, put back"],
  ["nested/change-one-line", "let b = [\"numbers\": [2, 4]]", "b[\"numbers\"] = (b[\"numbers\"] ?? []) + [6]"],
  ["nested/change-const-put-back.fails", "const b = [\"numbers\": a]", "take out, push, put back"],
  ["nested/change-original", "let a; const b = [\"numbers\": a]", "a.push(6), then print a and b"],
  ["nested/change-const-inside-let", "const a; let b = [\"numbers\": a]", "change b's list"],
  ["nested/change-record", "let b = Box(numbers: a)", "b.numbers.push(6)"],
  ["nested/change-const-record.fails", "const b = Box(numbers: [2, 4])", "b.numbers.push(6)"],
  ["nested/change-list", "let b = [a, [8]]", "b[0].push(6)"],
  ["nested/change-const-list.fails", "const b = [[2, 4], [8]]", "b[0].push(6)"],
  ["nested/change-copy", "const b; let copy = b", "change copy"],
  ["nested/change-if-let.fails", "if let nums = b[\"numbers\"]", "nums.push(6)"],
];

// Spec 2.2 and 14.2: the methods that change a collection, and those that
// only read. The samples methods-change and methods-const.fails check the
// first column; methods-read checks the second, all but `enumerate`, which
// `kavach run --native` doesn't compile yet (for/index shows it).
const methodKinds: [type: string, change: React.ReactNode, read: React.ReactNode][] = [
  [
    "List",
    <><C>push</C>, <C>pop</C>, <C>items[i] = v</C>, <C>+=</C></>,
    <><C>len</C>, <C>is_empty</C>, <C>is_full</C>, <C>get</C>, <C>first</C>, <C>last</C>, <C>contains</C>, <C>index_of</C>, <C>count</C>, <C>find</C>, <C>min</C>, <C>max</C>, <C>sum</C>, <C>map</C>, <C>filter</C>, <C>flat_map</C>, <C>any</C>, <C>all</C>, <C>enumerate</C>, <C>sort</C>, <C>sort_by</C>, <C>reverse</C>, <C>take</C>, <C>skip</C>, <C>slice</C>, <C>join</C>, <C>+</C></>,
  ],
  [
    "Map",
    <><C>m[k] = v</C>, <C>remove</C></>,
    <><C>m[k]</C>, <C>get</C>, <C>len</C>, <C>is_empty</C>, <C>is_full</C>, <C>contains_key</C>, <C>keys</C>, <C>values</C></>,
  ],
  ["Set", <><C>add</C>, <C>remove</C></>, <><C>contains</C>, <C>len</C>, <C>is_empty</C>, <C>is_full</C>, <C>to_list</C></>],
  [
    "Ring",
    <><C>push</C></>,
    <><C>len</C>, <C>is_empty</C>, <C>is_full</C>, <C>first</C>, <C>last</C>, <C>get</C>, <C>contains</C>, <C>to_list</C></>,
  ],
  ["String", <>none: text never changes</>, <>every method, which gives new text</>],
];

function ChangeTable() {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">With</th>
            <th className="py-2 pr-4 font-bold">Doing</th>
            <th className="py-2 font-bold">Result</th>
          </tr>
        </thead>
        <tbody>
          {changeCases.map(([name, setup, step]) => {
            const out = loadSample(name).output ?? "";
            const refused = name.endsWith(".fails");
            const lines = out.split("\n").map((l) => l.replace(/^error: /, ""));
            return (
              <tr key={name} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{setup}</C></td>
                <td className="py-2 pr-4"><C>{step}</C></td>
                <td className="py-2 text-muted">
                  <strong className="text-ink">{refused ? "Refused: " : "Works: "}</strong>
                  {lines.map((l, i) => (
                    <span key={i}>{i > 0 && ", then "}<C>{l}</C></span>
                  ))}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

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
      <H2>A real shape: records, maps, lists and functions</H2>
      <P>
        Real data mixes kinds: a school has a name (text) and its classes (a map). Written as one map, as in JSON or
        Python, it is refused, as all the values of a map have one type:
      </P>
      <Ex name="nested/school-map.fails" />
      <P>Give each kind of thing its own place instead:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <Link href="/docs/types/records/">record</Link> for a thing with fixed fields: a <C>School</C> has a <C>name</C> and <C>rooms</C>.</li>
        <li>A map for things found by name: rooms by class, marks by subject.</li>
        <li>A list for things in order: students, and their marks.</li>
        <li>A <Link href="/docs/functions/functions-as-values/">function as a value</Link> where each entry has its own rule: <C>[String: fn(Int) -&gt; Int]</C> holds each class&apos;s fee rule, a named function or a lambda.</li>
      </ul>
      <Ex name="nested/school" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>school.rooms</C> is a record&apos;s field, so reading it never gives an optional. <C>room.marks[&quot;math&quot;]</C> is a map&apos;s, so it does: <C>?? []</C> gives an empty list for a missing subject.</li>
        <li><C>if let fee = fees[class_name]</C> runs its block only when the class has a fee rule, then <C>fee(…)</C> calls it.</li>
        <li>A record checks its fields when the program is built: a typo such as <C>room.teachr</C> is a compile error, where a map would give <C>none</C> while running.</li>
      </ul>
      <Note>
        When such data comes from outside as JSON, decode it straight into records like these. See{" "}
        <Link href="/docs/standard-library/json/decoding-into-a-type/">Decoding into a type</Link>.
      </Note>
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
      <H2>Changing what&apos;s inside: let and const</H2>
      <P>
        Can a list inside a map be changed? It depends on two things: whether the outer value is a <C>let</C>, and
        whether reaching the inner value gives an <Link href="/docs/types/optionals/">optional</Link>. This is
        refused twice over:
      </P>
      <Ex name="nested/change-const-map.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>b[&quot;numbers&quot;]</C> is a <C>[Int]?</C>, as the key may be missing, and <C>none</C> has no <C>push</C>. The compiler says this first.</li>
        <li><C>b</C> is a <C>const</C>, so nothing inside it can change at all, however deep.</li>
      </ul>
      <P>With a <C>let</C>, take the list out, change it and put it back. Or do it in one line, with <C>+</C>:</P>
      <Ex name="nested/change-let-put-back" />
      <P>Every case, each one a checked sample:</P>
      <ChangeTable />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>const</C> never changes, at any depth. To change one, copy it into a <C>let</C>: the copy changes, and the original stays as it was.</li>
        <li>In a <C>let</C>, a list&apos;s item (<C>b[0]</C>) and a record&apos;s field (<C>b.numbers</C>) change in place. A map&apos;s value is an optional: take it out and put it back.</li>
        <li>Putting <C>a</C> into <C>b</C> puts in a copy. Changing <C>a</C> later leaves <C>b</C> as it was, and changing <C>b</C> leaves <C>a</C>.</li>
        <li>A name from <C>if let</C> is a <C>const</C>. To change what it holds, copy it into a <C>let</C>, change that, and put it back.</li>
      </ul>
      <H2>Which methods change a collection</H2>
      <P>
        Only a few methods change the collection they are called on. They need a <C>let</C>, at every depth. Every
        other method only reads: it works on a <C>const</C> too, and gives a new value.
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Type</th>
              <th className="py-2 pr-4 font-bold">Change it (need a let)</th>
              <th className="py-2 font-bold">Only read (work on a const)</th>
            </tr>
          </thead>
          <tbody>
            {methodKinds.map(([type, change, read]) => (
              <tr key={type} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{type}</C></td>
                <td className="py-2 pr-4">{change}</td>
                <td className="py-2 text-muted">{read}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>Every reading method, on the fields of a <C>const</C> record. None of them changes it:</P>
      <Ex name="nested/methods-read" />
      <P>Every changing method, on the fields of a <C>let</C> record, in place:</P>
      <Ex name="nested/methods-change" />
      <P>On a <C>const</C>, the compiler refuses each of these lines, one error for each. The first:</P>
      <Ex name="nested/methods-const.fails" />
      <P>
        Reading through a map&apos;s lookup needs <C>??</C> first, as changing does: the value may be missing.
      </P>
      <Ex name="nested/methods-map-read.fails" />
      <P>
        <C>sort()</C> and <C>reverse()</C> give a sorted or reversed copy and leave the list as it was. So a line
        that throws the copy away would look like it sorted the list, and doesn&apos;t: it is refused. Write{" "}
        <C>numbers = numbers.sort()</C>.
      </P>
      <Ex name="nested/methods-sort.fails" />
      <P>
        What each method does is in{" "}
        <Link href="/docs/reference/built-in-methods/">Built-in Methods</Link>.
      </P>
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
