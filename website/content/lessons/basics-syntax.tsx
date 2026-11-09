import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- Syntax ----------

export const syntaxDoc: LessonDoc = {
  lead: "How a Kavach file is written: one statement per line, blocks in braces, no semicolons.",
  simple: <>Syntax is the grammar of a programming language. Just as a sentence needs its words in order and a full stop, code needs its own rules so the computer can read it. This page gives Kavach&apos;s few rules.</>,
  minutes: 4,
  before: ["start/your-first-program"],
  body: (
    <>
      <Ex name="syntax/main" />
      <H2>The rules in short</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A file ends in <C>.kav</C>. A program starts at <C>fn main()</C>.</li>
        <li>One statement per line. There are no semicolons.</li>
        <li>Braces <C>{"{ }"}</C> group lines into a block: the body of a function, a loop or an <C>if</C>.</li>
        <li>Text goes in double quotes; a single character in single quotes.</li>
        <li>Indent with four spaces. <C>kavach fmt</C> does it for you.</li>
      </ul>
      <Note>
        Kavach is case-sensitive: <C>total</C> and <C>Total</C> are two different names.
      </Note>
    </>
  ),
};

export const sourceFilesDoc: LessonDoc = {
  lead: "A statement ends at the end of its line, unless the line clearly isn't finished.",
  simple: <>Each line is like one sentence, and the end of the line is its full stop. If a line clearly stops halfway, say with a <C>+</C> or a comma, Kavach reads the next line as more of the same sentence.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="syntax/statements" />
      <P>A statement goes on to the next line when:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>the line ends with an operator such as <C>+</C>, or with <C>,</C> <C>(</C> <C>[</C> <C>{"{"}</C>;</li>
        <li>the next line starts with <C>.</C>, as in a chain of method calls.</li>
      </ul>
      <P>Inside <C>( … )</C> and <C>[ … ]</C>, line breaks never end a statement, so long lists and calls can spread over lines freely.</P>
    </>
  ),
};

export const mainFunctionDoc: LessonDoc = {
  lead: "A program starts at fn main(). In a single-file script, the lines at the top level are main.",
  simple: <><C>main</C> is the front door of your program: the computer always comes in there and starts with its first line. For a short script you don&apos;t need to write it: the lines of the file become <C>main</C> by themselves.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="syntax/main" />
      <P>
        <C>main</C> calls <C>greet</C> although <C>greet</C> is written below it: functions and types can be used
        anywhere in their file. Local variables are different: they exist only from their line on.
      </P>
      <H2>Scripts</H2>
      <P>For a quick script, leave <C>main</C> out. The lines at the top level become its body:</P>
      <Ex name="start/script" />
    </>
  ),
};

export const blocksDoc: LessonDoc = {
  lead: "A block { … } has a value: the value of its last line.",
  simple: <>A block is a group of lines kept together in braces, like a small calculation on scrap paper. Only the final answer, its last line, comes out; the notes used along the way are thrown away.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="values/sub-block" />
      <P>
        <C>total</C> gets the value of the block&apos;s last line, <C>base + tax</C>, and <C>base</C> and{" "}
        <C>tax</C> are gone after it. An <C>if</C> is a value the same way, so it can pick between two values in
        one line.
      </P>
    </>
  ),
};

export const namesDoc: LessonDoc = {
  lead: "Names are made of letters, digits and _, in snake_case for values and PascalCase for types.",
  simple: <>Names are the labels you put on things in your program, like labels on jars in a kitchen. Kavach asks you to write them in one agreed style, and <C>kavach fmt</C> can fix the style for you.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="syntax/names" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>snake_case</C> for variables, functions and modules: <C>total_price</C>.</li>
        <li><C>PascalCase</C> for types and their variants: <C>OrderStatus</C>, <C>Paid</C>.</li>
        <li>Letters from any script work: <C>नाम</C> is a fine name. A name can&apos;t start with a digit.</li>
      </ul>
      <P>The compiler enforces the style, and says exactly what to write:</P>
      <Ex name="syntax/names-style.fails" />
      <Note>
        You rarely see this error: <C>kavach fmt</C> renames the name and every use of it for you.
      </Note>
    </>
  ),
};

const keywords = [
  "agent", "and", "arena", "as", "assert", "break", "capability", "catch", "const", "continue", "deprecated",
  "else", "ensure", "example", "extend", "extern", "fail", "false", "fn", "for", "if", "in", "interface",
  "invariant", "let", "match", "none", "not", "or", "pub", "require", "return", "secret", "self", "spawn",
  "test", "true", "try", "type", "unsafe", "use", "uses", "var", "while", "with",
];

export const keywordsDoc: LessonDoc = {
  lead: "Keywords are the words the language itself uses. They can't be used as names.",
  simple: <>Some words have a fixed meaning in Kavach, such as <C>if</C>, <C>for</C> and <C>fn</C>. The language needs them for its own work, so they are reserved, like seats kept for someone: you can&apos;t take them for your own names.</>,
  minutes: 2,
  body: (
    <>
      <div className="flex flex-wrap gap-2">
        {keywords.map((k) => (
          <C key={k}>{k}</C>
        ))}
      </div>
      <P>
        <C>var</C> declares nothing: it is reserved only so that writing it gives a clear message pointing to{" "}
        <Link href="/docs/basics/variables/let/">let</Link>. A few more words, such as <C>result</C> in a contract,
        are keywords only in one place and ordinary names everywhere else.
      </P>
      <P>A keyword can&apos;t name a field either: the field could never be read, since <C>r.agent</C> can&apos;t be written.</P>
      <Ex name="syntax/keyword-field.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/01-basics.md" line={82} symbol="Keywords" label="spec 1.6 · the full list" />
        <SrcRef file="tests/conformance/02-types/a-keyword-cant-name-a-field.kav" label="a keyword as a field, tested" />
      </div>
    </>
  ),
};
