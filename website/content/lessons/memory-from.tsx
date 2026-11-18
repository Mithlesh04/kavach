import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Values, Not References › Coming from JavaScript and Python ----------
//
// The JavaScript and Python blocks are shown, not checked by check-samples.
// Each was run with node and python on 2026-11-18, and printed what is shown.

const SPEC = "docs/spec/06-memory.md";

/** A table of spec 6.1, found by its first column's heading. */
const table = (first: string) => {
  const t = specTables("06-memory.md", "6.1").find((x) => x.header[0] === first && x.rows.length > 0);
  if (!t) throw new Error(`spec 6.1: no table whose first column is "${first}"`);
  return t;
};
const compare = () => table("You write");
const onPurpose = () => table("You want");

export const fromJsPythonDoc: LessonDoc = {
  lead: "In JavaScript and Python, b = a gives one list two names. In Kavach, b gets its own value. Here is each case side by side.",
  simple: <>In JavaScript or Python, giving someone your list is like giving them a key to your house: whatever they move, you find moved. In Kavach, you give them a photocopy of the list. They can scribble on it all they like; your list stays as it was.</>,
  minutes: 7,
  before: ["memory/values-not-references"],
  body: (
    <>
      <P>
        In JavaScript, Python, Java and C#, a list or an object is a <em>reference</em>: <C>b = a</C> gives the same
        list a second name, so a change through <C>b</C> shows through <C>a</C>. In Kavach every type is a value, so{" "}
        <C>b</C> gets its own. Spec 6.1 sums it up:
      </P>
      <Table header={compare().header} rows={compare().rows} />
      <P>
        Underneath, nothing is copied until one side changes, so values cost little (
        <Link href="/docs/memory/automatic-memory/how-it-stays-fast/">how it stays fast</Link>). Each case, step by step:
      </P>
      <H2>Giving a list a second name</H2>
      <CodeBlock code={"const a = [1, 2, 3]\nconst b = a\nb.push(4)\nconsole.log(a)"} lang="javascript" file="assign.js" output="[ 1, 2, 3, 4 ]" />
      <P>In Kavach, <C>a</C> keeps its three items:</P>
      <Ex name="memory/values" />
      <H2>A list put in a dict</H2>
      <CodeBlock code={'a = [2, 4]\nd = {"numbers": a}\na.append(6)\nprint(d)'} lang="python" file="dict.py" output="{'numbers': [2, 4, 6]}" />
      <P>A Kavach dict keeps the value it was given:</P>
      <Ex name="memory/from-dict" />
      <SrcRef file="tests/conformance/06-memory/a-dict-keeps-the-value-it-was-given.kav" label="tested" />
      <H2>Rows made from one list</H2>
      <P>
        A grid made of the same row three times is one row three times in Python, so setting one cell sets it in every
        row (<a href="https://docs.python.org/3/faq/programming.html#how-do-i-create-a-multidimensional-list">Python FAQ</a>):
      </P>
      <CodeBlock code={"row = [0, 0]\ngrid = [row, row, row]\ngrid[0][0] = 5\nprint(grid)"} lang="python" file="grid.py" output="[[5, 0], [5, 0], [5, 0]]" />
      <P>In Kavach, the grid holds three rows of its own:</P>
      <Ex name="memory/from-grid" />
      <SrcRef file="tests/conformance/06-memory/rows-made-from-one-list-are-separate.kav" label="tested" />
      <H2>A function and the list it was given</H2>
      <P>
        In Python, a default list is made once and shared by every call, so it keeps growing call after call (
        <a href="https://docs.python.org/3/tutorial/controlflow.html#default-argument-values">Python tutorial</a>):
      </P>
      <CodeBlock code={"def add(item, into=[]):\n    into.append(item)\n    return into\n\nprint(add(1))\nprint(add(2))"} lang="python" file="default.py" output={"[1]\n[1, 2]"} />
      <P>A Kavach function can&apos;t change a value it was given. Trying is a compile error:</P>
      <Ex name="memory/from-default.fails" />
      <P>It gives back a new list instead, so each call starts from an empty one:</P>
      <Ex name="memory/from-default" />
      <P>
        To change the caller&apos;s list on purpose, mark the parameter <C>inout</C>. The function&apos;s signature
        shows it, and so does each call (<Link href="/docs/functions/inout-parameters/">inout parameters</Link>).
      </P>
      <H2>Copies that are only one level deep</H2>
      <P>
        In JavaScript, spread copies the outer object only, so a nested list is still shared (
        <a href="https://developer.mozilla.org/en-US/docs/Glossary/Shallow_copy">MDN</a>). React asks for
        state to be treated as read-only for this reason (<a href="https://react.dev/learn/updating-objects-in-state">react.dev</a>):
      </P>
      <CodeBlock code={'const order = { lines: ["pen"] }\nconst copy = { ...order }\ncopy.lines.push("ink")\nconsole.log(order.lines)'} lang="javascript" file="spread.js" output="[ 'pen', 'ink' ]" />
      <P>In Kavach a copy is a copy at every depth:</P>
      <Ex name="memory/from-nested" />
      <H2>Threads</H2>
      <P>
        Two threads changing one list at once is a data race: in one case the Go team describes, a buffer shared by
        mistake silently corrupted data (<a href="https://go.dev/blog/race-detector">Go blog</a>). In Kavach
        each task gets its own copy of what it reads, so there is nothing to race on. Only a <C>Shared</C> value is
        shared, and its <C>update</C> takes a lock (<Link href="/docs/memory/shared-state/">shared state</Link>).
      </P>
      <H2>When you want the sharing</H2>
      <P>Sometimes two names for one value is what you mean. Say so, and Kavach shares it:</P>
      <Table header={onPurpose().header} rows={onPurpose().rows} />
      <P>Two names for one list:</P>
      <Ex name="memory/share-names" />
      <P>A dict entry that follows a variable:</P>
      <Ex name="memory/share-dict" />
      <P>Rows that are one row, as in Python:</P>
      <Ex name="memory/share-rows" />
      <SrcRef file="tests/conformance/06-memory/a-shared-inside-a-dict-is-shared.kav" label="Shared in a dict, tested" />
      <Note>
        Why Kavach chose values: a value changes only through the name that holds it, so a function can be read on its
        own, by people and by AI tools alike, and tasks on many cores can&apos;t race on data they don&apos;t share.
        Swift made the same choice for its arrays, dictionaries and strings (
        <a href="https://www.swift.org/documentation/articles/value-and-reference-types.html">Swift.org</a>).
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 6.1 · values, not references" />
        <SrcRef file="docs/decisions/0003-memory-model.md" label="decision 0003 · the memory model" />
      </div>
    </>
  ),
};
