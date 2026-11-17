import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { RaceChart } from "@/components/race-chart";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Limits } from "./limits";
import { Table } from "./spec-table";

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
      <Note>Each form below has its own page: making strings, putting values in, joining, length, searching, splitting, tidying, and reading numbers.</Note>
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
        <li>In a plain string, <C>\t</C> is a tab, <C>\n</C> a new line, <C>\r</C> a carriage return (as protocols that end lines with <C>\r\n</C> need), <C>\&quot;</C> a quote.</li>
      </ul>
      <P>
        Raw text can go over many lines too. <C>r&quot;&quot;&quot; … &quot;&quot;&quot;</C> keeps every backslash
        and brace, and removes the shared indentation as <C>&quot;&quot;&quot;</C> does. It suits JSON, SQL and
        regular expressions written out in full:
      </P>
      <Ex name="strings/raw-many" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/01-basics/raw-text-over-many-lines.kav" label="raw text over many lines, tested" />
      </div>
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
      <P>
        Every string fills in its braces by itself, so there is no <C>f&quot;…&quot;</C> as in Python. Writing one is
        an error that says to drop the <C>f</C>:
      </P>
      <Ex name="strings/f-string.fails" />
      <P>The braces can hold a calculation too. To write a brace itself, put a backslash before it:</P>
      <Ex name="values/sub-interp" />
      <P>In text on one line, a <C>{"{"}</C> must close on that line. One left open is an error at the brace, with the fix:</P>
      <Ex name="strings/brace-open.fails" />
      <P>
        A <C>{"{…}"}</C> that holds only text gives a warning, because it is almost always a brace that needed{" "}
        <C>{"\\{"}</C>. Here the second line is one string, with the text <C> or t == </C> inside its braces, not two
        comparisons:
      </P>
      <Ex name="strings/brace-text.warns" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/01-basics/interpolation-closes-on-its-line.kav" label="tested" />
        <SrcRef file="tests/conformance/01-basics/interpolation-of-only-text-warns.kav" label="the warning, tested" />
      </div>
    </>
  ),
};

export const joinDoc: LessonDoc = {
  lead: "+ joins two strings; join() puts a list of strings together; repeat() repeats one.",
  simple: <>Joining text is like threading beads: you put pieces end to end to make one longer piece, from two pieces, from a whole list of them, or from one piece used again and again.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="strings/join" />
      <P>
        <C>join</C> works on a list of characters too, and <C>contains</C> takes a <C>Char</C> as well as a{" "}
        <C>String</C>:
      </P>
      <Ex name="strings/chars-join" />
      <SrcRef file="tests/conformance/14-standard-library/chars-join-and-text-contains-a-char.kav" label="tested" />
    </>
  ),
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
      <H2>Counting, the last one, and taking off an end</H2>
      <Ex name="strings/count" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>count(part)</C> says how many times <C>part</C> is there, not counting overlaps: <C>&quot;banana&quot;</C> holds <C>&quot;ana&quot;</C> once.</li>
        <li><C>last_index_of(part)</C> is <C>index_of</C> from the end: a file&apos;s name starts after the last <C>/</C>.</li>
        <li><C>remove_prefix</C> and <C>remove_suffix</C> take a part off only when it is there; otherwise the text stays as it was.</li>
      </ul>
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
      <H2>Words</H2>
      <P>
        <C>words()</C> cuts at every run of spaces, tabs and line breaks, so text typed with extra spaces gives no
        empty parts. <C>split(&quot; &quot;)</C> keeps an empty part between each two spaces:
      </P>
      <Ex name="strings/words" />
    </>
  ),
};

export const tidyDoc: LessonDoc = {
  lead: "capitalize, title, trim_start, trim_end, truncate and is_blank tidy text; pad_left and pad_right line it up in columns.",
  simple: <>Like tidying a form before you file it: a capital letter at the start of a name, no stray spaces, a title that fits the box, numbers lined up in a column. Each method gives a new, tidy string.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="strings/tidy" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>capitalize()</C> puts the first character in upper case and leaves the rest.</li>
        <li><C>title()</C> does that to each word, and puts the rest of each word in lower case.</li>
        <li><C>trim_start()</C> and <C>trim_end()</C> take white space off one side; <C>trim()</C> takes it off both.</li>
        <li><C>truncate(max)</C> keeps at most <C>max</C> characters; <C>is_blank()</C> says whether there is only white space, or nothing.</li>
      </ul>
      <H2>Padding</H2>
      <P>
        <C>pad_left(width)</C> adds spaces before text until it is <C>width</C> characters long, and{" "}
        <C>pad_right</C> adds them after. <C>fill:</C> picks another character. A bill in two columns:
      </P>
      <Ex name="strings/pad" />
      <P>Text already that long stays as it was. The width is at most 1,000,000.</P>
      <SrcRef file="tests/conformance/14-standard-library/text-pads-words-and-capitals.kav" label="tested" />
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

// ---------- Text views and builders ----------
//
// Spec 2.18. The two method tables are the spec's own, checked by their
// first rows, so a change there shows here.

const VIEWS = "02-types/18-text-views.md";

function ViewTable({ n, first }: { n: number; first: string }) {
  const t = specTables(VIEWS, "2.18")[n];
  if (!t || !t.rows[0][0].includes(first)) throw new Error(`spec 2.18: table ${n} no longer starts with ${first}`);
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const textViewsDoc: LessonDoc = {
  lead: "A StrView is part of a string with nothing copied, for reading text fast. A TextBuilder is text being made, changed in place.",
  simple: <>A view is a window onto part of a page: you read through it, and nothing is copied out. A builder is a notepad you keep writing on, instead of copying the whole page out again for every new word.</>,
  minutes: 5,
  before: ["basics/strings/splitting"],
  body: (
    <>
      <P>
        A <C>String</C> never changes, and <C>s.slice(a, b)</C> gives a copy. For most programs that is fast enough.
        For a parser that reads a big file, or code that builds long text a piece at a time, two built-in types skip
        the copies.
      </P>
      <H2>Views: reading without copying</H2>
      <P>
        <C>s.view()</C> is all of <C>s</C>, and <C>s.view(start, end)</C> characters <C>start..end</C> of it. Its{" "}
        <C>trim</C>, <C>split</C> and <C>lines</C> give views too, so a line is taken apart with no text copied:
      </P>
      <Ex name="strings/view" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A view compares with <C>==</C> and <C>&lt;</C> to another view or to a <C>String</C>, by its text, and prints its text.</li>
        <li><C>Int(v)</C>, <C>Float(v)</C> and the other conversions take a view, so a number is read without copying it out.</li>
        <li>A view keeps its whole string in memory. <C>v.to_string()</C> copies its text into a string of its own, and lets the big one go.</li>
      </ul>
      <ViewTable n={0} first="v.len()" />
      <H2>Builders: making text in place</H2>
      <P>
        <C>TextBuilder()</C> starts empty, <C>TextBuilder(text)</C> starts with <C>text</C>, and{" "}
        <C>TextBuilder.capacity(n)</C> has room for <C>n</C> bytes before it grows. It changes in place, so it is a{" "}
        <C>let</C>:
      </P>
      <Ex name="strings/builder" />
      <ViewTable n={1} first="b.push(text)" />
      <P>A builder in a <C>const</C> can&apos;t change:</P>
      <Ex name="strings/builder-const.fails" />
      <P>
        A builder is a value, like every other. A copy is a builder of its own, and a view of a builder keeps the text
        it had when it was taken:
      </P>
      <Ex name="strings/builder-copy" />
      <P>A position past the end is clamped, as for <C>slice</C>. A start after its end is a panic:</P>
      <Ex name="strings/builder-range.panics" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${VIEWS}`} label="spec 2.18 · text views and builders" />
        <SrcRef file="tests/conformance/02-types/text-views-read-without-copying.kav" label="views, tested" />
        <SrcRef file="tests/conformance/02-types/text-builders-change-in-place.kav" label="builders, tested" />
        <SrcRef file="tests/conformance/02-types/text-builder-changes-need-let.kav" label="let, tested" />
        <SrcRef file="tests/conformance/02-types/text-builder-start-after-end-panics.kav" label="the panic, tested" />
      </div>
    </>
  ),
};
