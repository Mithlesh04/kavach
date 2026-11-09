import { C } from "@/components/inline-code";
import { RaceChart } from "@/components/race-chart";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { Limits } from "./limits";

// ---------- Strings ----------

export const stringsDoc: LessonDoc = {
  lead: "A String is text: letters, digits, spaces, any script. Write it in double quotes.",
  simple: <>A string is any piece of text: a name, a message, a sentence in any language. Once made, it never changes; tidying it, such as turning it into capitals, gives you a new string.</>,
  minutes: 4,
  before: ["basics/variables"],
  body: (
    <>
      <Ex name="strings/overview" />
      <P>
        Braces inside text put a value in: <C>&quot;Hello, {"{name}"}!&quot;</C>. Methods do the rest:{" "}
        <C>len()</C> counts characters, <C>upper()</C> gives capitals.
      </P>
      <H2>Strings never change</H2>
      <P>
        <C>name.upper()</C> gives a new string and leaves <C>name</C> as it was. That is why the methods can be
        chained safely, one after another:
      </P>
      <Ex name="text/sub-clean" />
      <Note>Each form below has its own page: making strings, putting values in, joining, length, searching, splitting, and reading numbers.</Note>
      <H2>Limits and sizes</H2>
      <Limits id="text" />
    </>
  ),
  tryIt: {
    task: <>Take <C>&quot;  kavach  &quot;</C>, remove the spaces around it, and print it in capitals.</>,
    answer: <P><C>print(&quot;  kavach  &quot;.trim().upper())</C> prints <C>KAVACH</C>.</P>,
  },
  deeper: (
    <>
      <P>
        Because a string never changes, it can be shared instead of copied: handing text to a function or keeping
        it in a list doesn&apos;t copy its characters. Building, splitting and searching large text, against C++:
      </P>
      <RaceChart suite="loops" id="text" />
      <VerifyBlock suite="loops" id="text" />
    </>
  ),
};

export const makeStringsDoc: LessonDoc = {
  lead: "Plain strings, multi-line strings in triple quotes, and raw strings that take every character as written.",
  simple: <>Most text fits in plain quotes. A long note over several lines goes in triple quotes, and text such as a Windows folder path, where every backslash must stay exactly as typed, goes in a raw string.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="strings/make" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>&quot;&quot;&quot; … &quot;&quot;&quot;</C> holds several lines. The indentation they share is removed, so the text can sit neatly in your code.</li>
        <li><C>r&quot;…&quot;</C> is raw: a backslash or a brace is just a character. Handy for Windows paths.</li>
        <li>In a plain string, <C>\t</C> is a tab, <C>\n</C> a new line, <C>\&quot;</C> a quote.</li>
      </ul>
    </>
  ),
};

export const interpolationDoc: LessonDoc = {
  lead: "Write any expression in braces inside a string, and its value is put in its place.",
  simple: <>Like a form letter with blanks: the braces mark each blank, and when the text is made, each blank is filled with a value, such as a name or a total.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="text/sub-insert" />
      <P>The braces can hold a calculation too. To write a brace itself, put a backslash before it:</P>
      <Ex name="values/sub-interp" />
    </>
  ),
};

export const joinDoc: LessonDoc = {
  lead: "+ joins two strings; join() puts a list of strings together; repeat() repeats one.",
  simple: <>Joining text is like threading beads: you put pieces end to end to make one longer piece, from two pieces, from a whole list of them, or from one piece used again and again.</>,
  minutes: 2,
  body: <Ex name="strings/join" />,
};

export const lengthDoc: LessonDoc = {
  lead: "len() counts the characters of a string; slice() and chars() take it apart.",
  simple: <>Think of text as a row of letter tiles. You can count the tiles, take out a stretch of them, or pick them up one by one, and a letter from any language counts as one tile.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="strings/length" />
      <P>
        <C>len()</C> counts characters, not bytes, so text in any script is measured the same way.{" "}
        <C>slice(0, 3)</C> gives characters 0, 1 and 2.
      </P>
    </>
  ),
};

export const searchDoc: LessonDoc = {
  lead: "contains, starts_with and ends_with answer yes or no; index_of says where; replace swaps parts.",
  simple: <>Like Find and Replace in a word processor: ask whether a word is in the text, whether the text begins or ends with it, where it is, or swap it for another.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="text/sub-search" />
      <Ex name="strings/index" />
      <P><C>index_of</C> gives <C>none</C> when the part isn&apos;t there, so <C>?? -1</C> picks what to use then.</P>
    </>
  ),
};

export const splitDoc: LessonDoc = {
  lead: "split(sep) cuts a string into a list of strings; lines() cuts it at line breaks.",
  simple: <>A shopping list typed on one line, milk, eggs, bread, can be cut at each comma into separate items. That is splitting. Cutting at line breaks gives you one piece per line.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="text/sub-split" />
      <Ex name="strings/lines" />
    </>
  ),
};

export const toNumbersDoc: LessonDoc = {
  lead: "Int(text) reads a whole string as a number, and can fail; std.text finds numbers inside other text.",
  simple: <>The text &quot;42&quot; looks like a number, but it is only characters, like a price written on a label. Reading it as a number lets you do sums with it, and fails when the text isn&apos;t a number at all.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="types/sub-convert" />
      <P>To find numbers in the middle of text, such as an order number in a message, use <C>std.text</C>:</P>
      <Ex name="text/sub-find" />
    </>
  ),
};
