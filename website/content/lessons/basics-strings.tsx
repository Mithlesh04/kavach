import Link from "next/link";
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
  lead: "A String is text: letters, digits, spaces, any script. Write it in double quotes, or in backticks to fill in values.",
  simple: <>A string is any piece of text: a name, a message, a sentence in any language. Once made, it never changes; tidying it, such as turning it into capitals, gives you a new string.</>,
  minutes: 4,
  before: ["basics/variables"],
  body: (
    <>
      <Ex name="strings/overview" />
      <P>
        Text in backticks is a template, and braces in it put a value in: <C>{"`Hello, {name}!`"}</C>. Methods do the
        rest:{" "}
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
  lead: "Text in quotes is exactly what is written, on one line. Backticks make a template, the one way to write text over several lines.",
  simple: <>Text in quotes is taken exactly as you type it, backslashes and braces too, so a folder path or a pattern needs nothing special. When you want values filled in, a tab, a quote mark inside, or a long note over several lines, use backticks.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="strings/make" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>&quot;…&quot;</C> stays on one line, has no escapes and fills nothing in: a backslash and a brace are characters like any other.</li>
        <li>Text over several lines, or with a <C>&quot;</C> in it, goes in backticks.</li>
        <li>In a template, in backticks, <C>\t</C> is a tab, <C>\n</C> a new line, <C>\r</C> a carriage return (as protocols that end lines with <C>\r\n</C> need), <C>{"\\`"}</C> a backtick, <C>{"\\{"}</C> a brace and <C>{"\\\\"}</C> a backslash.</li>
      </ul>
      <P>
        So a path or a regular expression goes in quotes as it is, with nothing to escape. JSON has quote marks in
        it, so it goes in a template: write <C>{"\\{"}</C> for each opening brace and <C>{"\\\\"}</C> for each
        backslash. A <C>{"}"}</C> needs nothing:
      </P>
      <Ex name="strings/raw-many" />
      <H2>Text over lines</H2>
      <P>
        A template whose text starts on the line after its backtick spans lines. Its lines can be indented like
        the code around them:
      </P>
      <Ex name="strings/over-lines" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The line break right after the opening backtick is left out, and so is the one before a closing backtick on its own line.</li>
        <li>The indentation the lines share is removed. A line indented deeper keeps the rest, as <C>your order</C> does.</li>
        <li>Empty lines stay. <C>{"{name}"}</C> is filled in and escapes are read, as in every template; <C>&quot;</C> needs no escape.</li>
        <li>A template whose text starts on the same line as its backtick stays on that line. A line break in it is an error that says to start the text on the next line.</li>
      </ul>
      <P>
        Python writes text over lines in three quotes. Kavach has one form for it, backticks, so{" "}
        <C>&quot;&quot;&quot;</C> is an error that shows the backtick form:
      </P>
      <Ex name="habits/triple-quotes.fails" />
      <P>
        A string in quotes that looks as if it wanted a template gets a warning that says to use backticks: braces
        around a name in scope, or a <C>\n</C> or <C>\t</C> that isn&apos;t part of a Windows path:
      </P>
      <Ex name="strings/looks-like-template.warns" />
      <P>
        There is no <C>r&quot;…&quot;</C>: text in quotes is raw already, and the message says to drop the <C>r</C>:
      </P>
      <Ex name="strings/r-string.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/01-basics.md" label="spec 1.7 · plain strings and templates" />
        <SrcRef file="docs/decisions/0131-plain-strings-and-backtick-templates.md" label="decision 0131" />
        <SrcRef file="docs/decisions/0143-text-over-lines-in-backticks.md" label="decision 0143 · text over lines" />
        <SrcRef file="tests/conformance/01-basics/a-quoted-string-is-plain.kav" label="plain, tested" />
        <SrcRef file="tests/conformance/01-basics/quoted-text-stays-on-one-line.kav" label="one line, tested" />
        <SrcRef file="tests/conformance/01-basics/a-template-over-lines-drops-common-indentation.kav" label="over lines, tested" />
        <SrcRef file="tests/conformance/01-basics/a-template-over-lines-keeps-empty-lines.kav" label="empty lines, tested" />
        <SrcRef file="tests/conformance/01-basics/a-template-on-its-backticks-line-stays-there.kav" label="on its line, tested" />
        <SrcRef file="tests/conformance/01-basics/json-over-lines-in-a-template.kav" label="JSON over lines, tested" />
        <SrcRef file="tests/conformance/01-basics/triple-quotes-are-not-kavach.kav" label="no triple quotes, tested" />
        <SrcRef file="tests/conformance/01-basics/a-quoted-string-that-looks-like-a-template-warns.kav" label="the warning, tested" />
        <SrcRef file="tests/conformance/01-basics/r-strings-are-not-kavach.kav" label="no r, tested" />
      </div>
    </>
  ),
};

export const interpolationDoc: LessonDoc = {
  lead: "Write text in backticks, a template, and any expression in braces inside it is put in its place.",
  simple: <>Like a form letter with blanks: the braces mark each blank, and when the text is made, each blank is filled with a value, such as a name or a total. The backticks say the text has blanks.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="text/sub-insert" />
      <P>
        Backticks are the only way to fill in values: there is no <C>f&quot;…&quot;</C> as in Python. Writing one is
        an error that says to use backticks:
      </P>
      <Ex name="strings/f-string.fails" />
      <P>The braces can hold a calculation too. To write a brace itself in a template, put a backslash before it:</P>
      <Ex name="values/sub-interp" />
      <P>
        A template may go over several lines when its text starts on the line after the backtick, and the
        indentation the lines share is removed (more in{" "}
        <Link href="/docs/basics/strings/making-strings/">Making strings</Link>):
      </P>
      <Ex name="strings/template-lines" />
      <P>In a template, a <C>{"{"}</C> must close on its own line. One left open is an error at the brace:</P>
      <Ex name="strings/brace-open.fails" />
      <P>
        A <C>{"{…}"}</C> that holds only text gives a warning, because it is almost always a brace that needed{" "}
        <C>{"\\{"}</C>. Here the first template runs on to the last backtick, with the template{" "}
        <C>{"` or t == `"}</C> inside its braces, not two comparisons:
      </P>
      <Ex name="strings/brace-text.warns" />
      <P>
        A package&apos;s tag may go right before a template: <C>{"sql`… where owner = {owner}`"}</C>. Then each{" "}
        <C>{"{…}"}</C> is a parameter the tag writes its own way, as <C>$1</C>, never pasted into the text. See{" "}
        <Link href="/docs/tools/tagged-literals/">Tagged Literals</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/01-basics/string-interpolation.kav" label="templates, tested" />
        <SrcRef file="tests/conformance/01-basics/a-template-spans-lines.kav" label="over lines, tested" />
        <SrcRef file="tests/conformance/01-basics/interpolation-closes-on-its-line.kav" label="closes on its line, tested" />
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
  lead: "len() counts the characters of a string; a slice such as word[0:3], and chars(), take it apart.",
  simple: <>Think of text as a row of letter tiles. You can count the tiles, take out a stretch of them, or pick them up one by one, and a letter from any language counts as one tile.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="strings/length" />
      <P>
        <C>len()</C> counts characters, not bytes, so text in any script is measured the same way.{" "}
        <C>word[0:3]</C> gives characters 0, 1 and 2, and <C>word[0]</C> the first, as a <C>Char</C>. More in{" "}
        <Link href="/docs/collections/indexing-and-slicing/">Indexing and Slicing</Link>.
      </P>
      <H2>What kind of character</H2>
      <P>A <C>Char</C> says what kind of character it is, in any script:</P>
      <CharKinds />
      <Ex name="strings/char-kinds" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>is_digit()</C> is only <C>0</C> to <C>9</C>, the digits <C>Int(text)</C> reads. So <C>&apos;٣&apos;</C>, the Arabic-Indic three, gives <C>false</C>.</li>
        <li>Letters, white space and case follow Unicode, so <C>&apos;ß&apos;</C>, a lower-case letter, isn&apos;t upper-case.</li>
        <li>A <C>Char</C> has no <C>upper()</C>, as one character can become two: change the text&apos;s case instead, <C>`{"{c}"}`.upper()</C>, which turns <C>ß</C> into <C>SS</C>.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/02-built-in-methods.md" label="spec 14.2 · Characters" />
        <SrcRef file="tests/conformance/14-standard-library/a-char-says-what-kind-it-is.kav" label="tested" />
      </div>
    </>
  ),
};

/** Spec 14.2's table of a Char's kind checks. */
function CharKinds() {
  const t = specTables("14-standard-library/02-built-in-methods.md", "14.2").find((x) => x.header[0] === "Method" && x.rows.some((r) => r[0].includes("is_digit")));
  if (!t) throw new Error("spec 14.2: no table of a Char's kind checks");
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

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
      <H2>Empty text</H2>
      <P>
        Empty text has no lines, and an empty part is found everywhere, at once: <C>contains(&quot;&quot;)</C> is{" "}
        <C>true</C> and <C>index_of(&quot;&quot;)</C> is <C>0</C>, as in Python and JavaScript:
      </P>
      <Ex name="strings/empty" />
      <P>
        <C>split(&quot;&quot;)</C> has no separator to cut at, so it stops the program and says to use{" "}
        <C>chars()</C>, as Python&apos;s <C>split</C> refuses one:
      </P>
      <Ex name="strings/split-empty.panics" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/02-built-in-methods.md" label="spec 14.2 · text methods" />
        <SrcRef file="tests/conformance/14-standard-library/text-methods-on-empty-text.kav" label="empty text, tested" />
        <SrcRef file="tests/conformance/14-standard-library/split-on-empty-text-panics.kav" label="split on nothing, tested" />
      </div>
    </>
  ),
};

export const tidyDoc: LessonDoc = {
  lead: "capitalize, title, trim_start, trim_end and is_blank tidy text, and a slice cuts it short; pad_left and pad_right line it up in columns.",
  simple: <>Like tidying a form before you file it: a capital letter at the start of a name, no stray spaces, a title that fits the box, numbers lined up in a column. Each method gives a new, tidy string.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="strings/tidy" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>capitalize()</C> puts the first character in upper case and leaves the rest.</li>
        <li><C>title()</C> does that to each word, and puts the rest of each word in lower case.</li>
        <li><C>trim_start()</C> and <C>trim_end()</C> take white space off one side; <C>trim()</C> takes it off both.</li>
        <li><C>s[:max]</C>, a slice, keeps at most <C>max</C> characters; <C>is_blank()</C> says whether there is only white space, or nothing.</li>
      </ul>
      <H2>Padding</H2>
      <P>
        <C>pad_left(width)</C> adds spaces before text until it is <C>width</C> characters long, and{" "}
        <C>pad_right</C> adds them after. <C>fill:</C> picks another character. A bill in two columns:
      </P>
      <Ex name="strings/pad" />
      <P>Text already that long stays as it was. The width is at most 1,000,000.</P>
      <P>
        A negative count or width counts as 0, as it does everywhere a method takes one, such as <C>repeat</C> and{" "}
        <C>pad_left</C>. In a slice, a number below 0 counts as before the first character, so <C>s[:n]</C> with{" "}
        <C>n</C> of -1 is empty:
      </P>
      <Ex name="strings/negative-counts" />
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
        A <C>String</C> never changes, and a slice <C>s[a:b]</C> gives a copy. For most programs that is fast enough.
        For a parser that reads a big file, or code that builds long text a piece at a time, two built-in types skip
        the copies.
      </P>
      <H2>Views: reading without copying</H2>
      <P>
        <C>s.view()</C> is all of <C>s</C>, and a slice of a view is a view: <C>s.view()[start:end]</C> is characters{" "}
        <C>start..end</C> of <C>s</C>, with nothing copied. Its <C>trim</C>, <C>split</C> and <C>lines</C> give views
        too, so a line is taken apart with no text copied:
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
      <P>A position past the end is clamped, as in a slice. A start after its end is a panic:</P>
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
