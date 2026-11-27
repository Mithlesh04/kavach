import Link from "next/link";
import { Explainer } from "@/components/explain/explainer";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- Output ----------

export const outputDoc: LessonDoc = {
  lead: "print(value) writes a value to the screen, followed by a new line.",
  simple: <>Printing means showing something on the screen: it is how a program talks back to you. Without it, a program could work out an answer, but you would never see it.</>,
  minutes: 3,
  before: ["start/your-first-program"],
  body: (
    <>
      <P>
        <C>print</C> takes one value of any kind: text, a number, a list. Each call writes one line.
      </P>
      <Ex name="output/print" />
      <Note>
        Text goes between double quotes: <C>&quot;Hello&quot;</C>. To put a value inside text, write the text in
        backticks and the value in braces: <C>{"`Total: {total}`"}</C>. The Strings page covers this in full.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Print three lines: your name, your age as a number, and a list of two favourite foods.</>,
    answer: (
      <P>
        Three calls, one per line: <C>print(&quot;Asha&quot;)</C>, <C>print(31)</C>,{" "}
        <C>print([&quot;dosa&quot;, &quot;mango&quot;])</C>.
      </P>
    ),
  },
};

export const printDoc: LessonDoc = {
  lead: "Each print call writes exactly one line to standard output.",
  simple: <>Each <C>print</C> is like writing one line in a notebook: every call starts a fresh line. Writing to the screen is something code must be allowed to do, and in Kavach every function is allowed it unless the project says otherwise.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="output/print" />
      <P>
        Printing uses the <C>Console</C> capability. It is a{" "}
        <Link href="/docs/agents/capabilities/default-capabilities/">default capability</Link>: every function may
        print without writing <C>uses</C>, unless the project&apos;s <C>kavach.toml</C> takes it away. An agent is
        the one exception: it may print only when its <C>can</C> lists <C>Console</C>.
      </P>
      <H2>What the machine does</H2>
      <P>
        A program can&apos;t draw on the screen itself. It hands its text to the operating system, which passes it
        to the terminal. <C>print</C> first builds the whole line in memory, then hands it over in one go:
      </P>
      <Explainer id="print" />
      <P>
        In detail: the interpreter (<C>kavach run</C>), and native code with tasks on several cores, build each line
        in a buffer of their own before writing it. Native code on one core writes each piece as it comes, into the
        standard output&apos;s own buffer, which holds the bytes until the newline. With one core, no other task&apos;s
        output can land in the middle, so the result is the same. When a value of the line has to wait, for a
        task for example, the compiler works out every value first, so a line never stops half way.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="compiler/interp/src/tasks.rs" line={424} symbol="flush" />
        <SrcRef file="runtime/native/src/rt.rs" line={279} symbol="kv_print_end" />
        <SrcRef file="compiler/kir/src/whole_lines.rs" line={18} symbol="print_whole_lines" />
      </div>
    </>
  ),
};

export const valuesPrintDoc: LessonDoc = {
  lead: "print shows a value the way you would write it in code, so the output is never ambiguous.",
  simple: <>Printed values never leave you guessing: a number with a fraction always shows its point, as in <C>7.0</C>, and text inside a list keeps its quotes. What you see is what you would type in code.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="output/values" />
      <P>
        A <C>Float</C> always shows a point (<C>7.0</C>, never <C>7</C>), so you can tell it from a whole number. A{" "}
        <C>Decimal</C> keeps its digits (<C>499.00</C>). Text inside a list is quoted, so{" "}
        <C>[&quot;a, b&quot;]</C> can&apos;t be mistaken for two items. A record shows its type and fields.
      </P>
      <P>
        Quoted text shows as written, accents and every script too. Only what you couldn&apos;t see or tell apart
        is escaped: a quote, a backslash, a tab, a line break, and invisible characters such as a zero-width space:
      </P>
      <Ex name="output/as-written" />
      <SrcRef file="tests/conformance/01-basics/text-in-collections-shows-as-written.kav" label="as written, tested" />
      <H2>Your own form</H2>
      <P>
        A type chooses how it shows by implementing <C>Display</C>, an interface every file has without{" "}
        <C>use</C>: one method, <C>fn display(self) -&gt; String</C>. That form is used everywhere the value is shown:{" "}
        <C>print</C>, <C>{"{…}"}</C> in text, <C>String(v)</C>, and inside lists, dicts and other records:
      </P>
      <Ex name="output/display" />
      <SrcRef file="tests/conformance/01-basics/a-type-shows-as-its-display-gives.kav" label="Display everywhere, tested" />
    </>
  ),
};

export const debugDoc: LessonDoc = {
  lead: "debug(value) prints a value with its file and line while you develop, and disappears from release builds.",
  simple: <><C>debug</C> is like a sticky note you put on your work while you fix it: it shows a value and says where it came from. When you build the finished program, the sticky notes are taken off for you.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="output/debug" />
      <P>
        Only <C>done</C> shows above, because <C>debug</C> writes to standard error, not standard output. In a
        terminal you see both: the <C>debug</C> line names the file and line it came from, so you know which{" "}
        <C>debug</C> printed it.
      </P>
      <H2>Why not just print?</H2>
      <P>
        <C>debug</C> needs no capability, so you can drop it into any function. And <C>kavach build</C> leaves it
        out completely, argument and all: a forgotten <C>debug</C> never reaches your users.
      </P>
    </>
  ),
};

// ---------- Comments ----------

export const commentsDoc: LessonDoc = {
  lead: "Comments are notes for people: the compiler skips them.",
  simple: <>A comment is like a note in the margin of a book: it is for readers, and the computer ignores it. Use it to explain why the code is written the way it is.</>,
  minutes: 3,
  body: (
    <>
      <H2>Three kinds</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>{"// …"}</C> runs to the end of the line.</li>
        <li><C>{"/* … */"}</C> can run over several lines, or sit inside a line.</li>
        <li><C>{"/// …"}</C> is a doc comment: it describes the declaration below it, and editors show it on hover.</li>
      </ul>
      <Ex name="values/sub-comments" />
      <Note>
        A comment says <em>why</em> the code is as it is. What the code does, the code itself should say.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Put a whole <C>print</C> line aside without deleting it, so it doesn&apos;t run.</>,
    answer: <P>Wrap it in <C>{"/* … */"}</C>, or start the line with <C>{"//"}</C>.</P>,
  },
};
