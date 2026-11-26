import Link from "next/link";
import { C } from "@/components/inline-code";
import { MemorySteps } from "@/components/memory-steps";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- Concepts: the core design choices ----------
//
// The why, in depth: each page is one choice, as its decision record makes
// it: the problem, what other languages do, what Kavach does (with checked
// samples), what it costs, and where it was decided. The how is on the
// pages each one links to.

const ul = "m-0 flex list-disc flex-col gap-1.5 pl-5";

export const conceptValuesDoc: LessonDoc = {
  lead: "In Kavach, b = a gives b its own value. Two names never share one changing value by accident, and sharing underneath makes it cost nothing until one of them changes.",
  simple: <>When you hand someone a copy of a document, what they write on it doesn&apos;t change yours. Kavach treats every value that way, so a change in one place never shows up somewhere you didn&apos;t expect.</>,
  minutes: 6,
  before: ["memory/values-not-references"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        In Python, JavaScript, Java and C#, <C>b = a</C> on a list makes two names for one list. A function that
        changes the list it was given changes the caller&apos;s list too. The bug shows up far from its cause: a list
        that changed &quot;by itself&quot;, a cache that changed under its readers, a default argument that remembers
        last time&apos;s calls.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Copy on purpose, with <C>list(a)</C>, <C>[...a]</C> or <C>.clone()</C>, and remember to every time.</li>
        <li>Freeze values, as JavaScript&apos;s <C>Object.freeze</C> does, a layer deep.</li>
        <li>Make collections values, as Swift does for its arrays and dictionaries, copied only when changed.</li>
        <li>Track who may change what, as Rust&apos;s borrow checker does, at the cost of learning its rules.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>Every value is a value, as in Swift: assigning or passing it never shares a changing value.</P>
      <Ex name="memory/values" />
      <P>
        It doesn&apos;t cost a copy each time. Underneath, both names hold one object until one of them changes,
        and only then is it copied. With no other holder, a change is made in place:
      </P>
      <MemorySteps name="copy" />
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>When two parts of a program really must see one changing value, you say so: <Link href="/docs/memory/shared-state/">Shared</Link>. That is the point: the sharing is visible in the code.</li>
        <li>A count on each shared object, kept while the program runs. The compiler removes most of the updates.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0003-memory-model.md" label="decision 0003 · the memory model" />
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.1 · values" />
      </div>
    </>
  ),
};

export const conceptErrorsDoc: LessonDoc = {
  lead: "A function that can fail says so in its type, and the caller must handle it. A bug, such as an index out of range, is a panic instead: it stops the task where the bug is.",
  simple: <>Some things go wrong because the world is messy: a file is missing, a network is down. Others go wrong because the code is wrong. Kavach keeps the two apart: the first kind is a value you must handle, the second stops the program at the mistake.</>,
  minutes: 6,
  before: ["errors"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        With exceptions, any call might fail, and nothing in its signature says so. The paths where things go wrong
        are the ones nobody sees in review, and in code that moves money or runs an agent, those are the paths that
        matter.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Python, JavaScript and C# throw exceptions that the signature doesn&apos;t show. Java&apos;s checked exceptions show some of them.</li>
        <li>Go returns errors as values, but a call whose error is never looked at still compiles.</li>
        <li>Rust returns a <C>Result</C> that must be handled, with <C>?</C> to pass it on.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>A function that can fail writes <C>! ErrorType</C>, and its errors are values of that type:</P>
      <Ex name="errors/declare" />
      <P>A call that can fail and isn&apos;t handled is a compile error:</P>
      <Ex name="errors/unhandled.fails" />
      <P><C>try</C> passes an error on, so the code that goes right stays short:</P>
      <Ex name="errors/try" />
      <P>A bug isn&apos;t an error value. It is a panic, which stops the task at the line of the mistake:</P>
      <Ex name="errors/explicit.panics" />
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>Every caller says what happens on failure. In a script, <C>! Error</C> on <C>main</C> catches everything.</li>
        <li>Adding a failure deep down changes the signatures above it, which is the change a reviewer should see.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0007-errors-as-values.md" label="decision 0007 · errors are values" />
        <SrcRef file="docs/spec/05-errors.md" label="spec 5 · errors" />
      </div>
    </>
  ),
};

export const conceptCapabilitiesDoc: LessonDoc = {
  lead: "A function's type says what it may touch: files, the network, the clock, a payment API. A function that says nothing touches nothing, and the compiler holds every function to what it says.",
  simple: <>Before you let a program, or an AI agent, run on your computer, you want to know what it can do. In Kavach, every function says that in its first line, and it can&apos;t do anything else.</>,
  minutes: 6,
  before: ["agents/capabilities"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        In most languages, any function anywhere can open a socket, read a file or call a payment API. A library
        can&apos;t promise otherwise, because the next function it calls might. For code written by people you
        don&apos;t know, or by AI, &quot;what can this do?&quot; has no answer short of reading all of it.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Check permissions while running, for the whole process, as Deno does: found in production, not in review.</li>
        <li>Track effects in types, as Koka and Unison do, with effect handlers that take time to learn.</li>
        <li>Most do neither: any function may do anything.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        A function lists what it touches with <C>uses</C>. A <C>pub</C> function must write it, so a module&apos;s
        outside shows what it does:
      </P>
      <Ex name="agents/cap-pub.fails" />
      <P>A function with no <C>uses</C> is pure: it can only compute, so what it does is all in front of you:</P>
      <Ex name="agents/cap-pure" />
      <ul className={ul}>
        <li>Outside systems are capability types with implementations you can swap, so a test passes a fake.</li>
        <li>An agent is given only what its <C>can</C> lists, with <C>limit</C>s and <C>approve</C> rules checked while it runs (see <Link href="/docs/concepts/agents-with-limits/">Agents with Limits</Link>).</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>A network call added deep down changes the public signatures above it. That is deliberate: it is exactly the change a reviewer should see.</li>
        <li>Private functions may leave <C>uses</C> out, and the compiler works it out, so small programs stay short.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0008-capabilities-in-signatures.md" label="decision 0008 · capabilities in signatures" />
        <SrcRef file="docs/spec/08-capabilities-and-agents/README.md" label="spec 8 · capabilities" />
      </div>
    </>
  ),
};

export const conceptMemoryDoc: LessonDoc = {
  lead: "Kavach frees memory without a garbage collector and without a borrow checker: the compiler finds each value's last use, a count kept while running frees it at that moment, and values with one holder change in place.",
  simple: <>Memory is like desk space: a program takes some for each value and must give it back. Kavach gives it back the moment a value is no longer used, with no pause to clean up later and no rules for you to learn.</>,
  minutes: 7,
  before: ["memory/automatic-memory"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        Memory must be given back when a value is no longer needed. Doing it by hand is fast and easy to get wrong.
        Doing it automatically usually means either pauses while the program runs, or rules the programmer must
        learn and satisfy.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>A tracing garbage collector, as in Java, Go and C#: easy to use, with pauses and extra memory.</li>
        <li>A borrow checker, as in Rust: fast and safe, with a learning curve.</li>
        <li>By hand, as in C and Zig: full control, and unsafe by default.</li>
        <li>Counting holders, as Swift does, with the cost of every count update.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        It counts holders, as Swift does, and the compiler removes most of the counting. It finds each
        value&apos;s last use while compiling and hands the value over there instead of sharing it. So a value
        is freed at once when nothing holds it any more:
      </P>
      <MemorySteps name="share" />
      <ul className={ul}>
        <li>No pauses: memory is freed at a known point, when its last holder is done.</li>
        <li>No rules to satisfy: the compiler does the ownership work.</li>
        <li>For full control there are three more levels, opt-in: <Link href="/docs/memory/shared-state/">Shared</Link>, arenas, and <C>unsafe</C> pointers that <C>kavach audit</C> lists.</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>A count on each shared object, kept while running.</li>
        <li>Two values that hold each other can only be made through <C>Shared</C>, and a cycle left behind is reported.</li>
        <li>The work moves into the compiler, which is the hard part to build.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0003-memory-model.md" label="decision 0003 · the memory model" />
        <SrcRef file="docs/spec/06-memory.md" label="spec 6 · memory" />
      </div>
      <P>
        Every case, step by step: <Link href="/docs/memory/automatic-memory/memory-step-by-step/">Memory, Step by
        Step</Link>.
      </P>
    </>
  ),
};
