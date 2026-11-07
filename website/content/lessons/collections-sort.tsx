import Link from "next/link";
import { C } from "@/components/inline-code";
import { RadixDemo } from "@/components/radix-demo";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// How each type sorts (spec 1.8, Order), the same in the interpreter and in
// native code; the samples on these pages run on both with the same output.
const orders: [type: string, order: string, page: string][] = [
  ["Int, Int8 … UInt64", "smallest first; negative numbers before 0", "numbers"],
  ["Float, Float32", "smallest first; NaN after every number; 0.0 and -0.0 are equal", "numbers"],
  ["Decimal, BigInt", "by exact value, however many digits", "numbers"],
  ["String", "by Unicode code point, character by character: uppercase before lowercase, accented letters after z", "text"],
  ["Char", "by Unicode code point", "text"],
  ["Bool", "false before true", "by-a-key"],
  ["Tuples", "item by item: by the first item, then the second when the first are equal, and so on", "two-keys"],
  ["Records, unions", "no order of their own: use sort_by with a key", "by-a-key"],
];

// ---------- Sorting ----------

export const sortingDoc: LessonDoc = {
  lead: "list.sort() gives a sorted copy of a list of numbers, text, characters, Bools or tuples. sort_by(key) sorts anything else, by a key you choose.",
  simple: <>Sorting puts things in order, like lining children up by height or books by title. Numbers and text already have an order; for anything else, you say what to order by.</>,
  minutes: 6,
  before: ["collections/lists"],
  body: (
    <>
      <Ex name="sort/numbers" />
      <P>
        Both give a new list and leave the original as it was. For the largest first, add <C>.reverse()</C>.
      </P>
      <H2>How each type sorts</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Items</th>
              <th className="py-2 pr-4 font-bold">Order</th>
              <th className="py-2 font-bold">More</th>
            </tr>
          </thead>
          <tbody>
            {orders.map(([type, order, page]) => (
              <tr key={type} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{type}</C></td>
                <td className="py-2 pr-4 text-muted">{order}</td>
                <td className="py-2"><Link href={`/docs/collections/sorting/${page}/`} className="whitespace-nowrap">See it →</Link></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Note>
        A list holds one type, so there is no &quot;mixed&quot; list to sort: <C>[1, &quot;a&quot;]</C> doesn&apos;t
        compile. Data of mixed kinds is a list of records or of a union, sorted by a key you choose.
      </Note>
      <H2>How whole numbers sort so fast</H2>
      <P>
        Comparing pairs of numbers takes about <C>n × log n</C> steps. For a long list of whole numbers, Kavach uses a
        radix sort instead: it places each number by one digit at a time, in a few passes over the list. Step through it:
      </P>
      <RadixDemo />
      <P>Which algorithm runs for which items, and why, is under &quot;Go deeper&quot; below.</P>
    </>
  ),
  deeper: (
    <>
      <H2>Which algorithm</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Sorting</th>
              <th className="py-2 pr-4 font-bold">In a program built by Kavach</th>
              <th className="py-2 font-bold">Why</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["Whole numbers and Char, under 512 items", "a comparison sort (Rust's unstable sort, a quicksort variant)", "for a short list, counting digits costs more than comparing; equal whole numbers can't be told apart, so keeping their order doesn't matter"],
              ["Whole numbers and Char, 512 items or more", "a radix sort, 8 bits of each number at a time", "a few passes over the list instead of n × log n comparisons; bits that are the same in every number are skipped"],
              ["… 65,536 items or more", "first split by the top 8 bits into 256 buckets", "each bucket is small enough to sort inside the processor's cache, where memory is fastest"],
              ["… 1,048,576 items or more", "the buckets are sorted on all cores at once", "the buckets don't depend on each other, so the result is the same in any order"],
              ["Floats, text, Decimal, BigInt, tuples", "a stable merge sort (Rust's standard sort)", "these need comparing (NaN, text by code point), and equal items, like 0.0 and -0.0, keep their order"],
              ["sort_by", "the keys are worked out once each, then sorted with the stable merge sort", "your key function runs once per item, not once per comparison, and equal keys keep their order"],
            ].map(([what, how, why]) => (
              <tr key={what} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-ink">{what}</td>
                <td className="py-2 pr-4 text-muted">{how}</td>
                <td className="py-2 text-muted">{why}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        <C>kavach run</C> uses the stable merge sort for everything; the results are the same. A million numbers, sorted
        and checked:
      </P>
      <Ex name="sort/large" />
      <P>
        How much faster the radix sort is, against C++&apos;s <C>std::sort</C>, is on{" "}
        <Link href="/docs/speed/fast-sorting/">Fast Sorting</Link>. Sorting runs on the CPU today, not on the GPU.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="runtime/native/src/radix.rs" label="the radix sort" />
        <SrcRef file="docs/spec/01-basics.md" label="spec 1.8 · order" />
        <SrcRef file="docs/spec/14-standard-library/02-built-in-methods.md" label="spec 14.2 · sort" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>What does <C>[&quot;banana&quot;, &quot;Apple&quot;, &quot;apple&quot;].sort()</C> give?</>,
      options: ['["Apple", "apple", "banana"]', '["apple", "Apple", "banana"]', '["banana", "Apple", "apple"]'],
      answer: 0,
      explain: "Text sorts by Unicode code point, and uppercase letters come before lowercase ones. For an order that ignores case, use sort_by(w => w.lower()).",
    },
  ],
};

export const sortNumbersDoc: LessonDoc = {
  lead: "Numbers sort smallest first, exactly: NaN goes last, and Decimal and BigInt sort by their full value.",
  simple: <>Numbers line up from smallest to largest, like prices on a rack from cheapest to dearest. Every number gets its exact place, even a very long one.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="sort/numbers" />
      <Ex name="sort/floats" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>NaN</C> (not a number) sorts after every number, so a list holding one still sorts.</li>
        <li><C>0.1 + 0.2</C> is exactly <C>0.3</C> in Kavach, so it sorts as equal to <C>0.3</C>.</li>
        <li><C>0.0</C> and <C>-0.0</C> are equal, and keep their order.</li>
      </ul>
      <Ex name="sort/exact" />
    </>
  ),
};

export const sortTextDoc: LessonDoc = {
  lead: "Text sorts by Unicode code point, character by character: the same on every machine, whatever its language settings.",
  simple: <>Text is sorted by the code number each character has, not as a dictionary would. So the order is the same on every computer, but capital letters come before small ones.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="sort/text" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Uppercase letters come before lowercase ones: <C>&quot;Apple&quot;</C> before <C>&quot;apple&quot;</C>.</li>
        <li>Letters with accents come after <C>z</C>: <C>É</C> is code point 201, <C>z</C> is 122.</li>
        <li>Digits are characters too: <C>&quot;file10&quot;</C> comes before <C>&quot;file9&quot;</C>, because <C>1</C> comes before <C>9</C>.</li>
      </ul>
      <P>To ignore case, sort by the lowercase text. Words equal apart from case keep their order:</P>
      <Ex name="sort/ignore-case" />
    </>
  ),
};

export const sortKeyDoc: LessonDoc = {
  lead: "sort_by(key) sorts by what key gives for each item: a field, or any number or text worked out from it.",
  simple: <>Like lining up runners by finishing time instead of by name: you pick the one thing to compare, and the list is ordered by it. Runners with the same time stay in the order they were in.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="sort/by-key" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>sort_by(.score)</C> sorts by a field. Equal scores keep their order: Zoe stays before Omar. This is a stable sort.</li>
        <li><C>p =&gt; -p.score</C> sorts the highest score first.</li>
        <li>The key can be a number, text, a character, a <C>Bool</C>, or a tuple of them.</li>
      </ul>
      <P><C>Bool</C>s sort <C>false</C> first, and tuples item by item:</P>
      <Ex name="sort/bools" />
      <P>A record has no single natural order, so neither <C>sort()</C> nor a record key is allowed; the compiler asks for a key it can order:</P>
      <Ex name="sort/records.fails" />
      <Ex name="sort/record-key.fails" />
    </>
  ),
};

export const sortTwoKeysDoc: LessonDoc = {
  lead: "To sort by two keys, give sort_by a tuple: (p.city, p.name) sorts by city, then by name within a city.",
  simple: <>Like a school list sorted by class, and inside each class by name. The first key decides the order, and the second only settles ties.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="sort/tuple-key" />
      <P>
        Tuples compare item by item, so people in the same city are sorted by name. <C>(-p.age, p.name)</C> sorts the
        oldest first, and people of the same age by name.
      </P>
      <P>
        The same result comes from two stable sorts, the less important key first. Useful when a key is worked out in a
        separate step:
      </P>
      <Ex name="sort/two-keys" />
    </>
  ),
};
