import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Files, H2, Note, P } from "./parts";

// ---------- Watching for NaN ----------
//
// Spec 5.9: `[run] watch_nan = true` (or --watch-nan) stops the program at
// the first float operation of its own code that gives NaN, and names it.

const TESTS = "tests/conformance/05-errors";

export const nanDoc: LessonDoc = {
  lead: "The NaN watch stops a program at the first float operation that gives NaN, and names it, instead of letting NaN spread through every sum it touches.",
  simple: <><C>NaN</C> means &quot;not a number&quot;: the answer to a sum that has none, such as 0 divided by 0. It quietly spoils every sum it touches, like one drop of ink in a glass of water. The NaN watch catches the drop as it falls.</>,
  minutes: 4,
  before: ["errors/panics", "basics/data-types/float-and-float32"],
  body: (
    <>
      <P>
        A float operation with no answer gives <C>NaN</C>, as IEEE 754 says: <C>0.0 / 0.0</C>, <C>inf - inf</C>,{" "}
        <C>math.sqrt(-1.0)</C>. The program goes on, and the <C>NaN</C> spreads, so a model&apos;s loss or a report&apos;s
        total turns into <C>NaN</C> far from the bug:
      </P>
      <Ex name="errors/nan-spreads" />
      <P>
        Turn the watch on with <C>watch_nan = true</C> under <C>[run]</C> in <C>kavach.toml</C>. The program now
        stops at the first operation that makes a <C>NaN</C>, names it with its operands, and says where it is:
      </P>
      <Files name="errors/nan-watch.panics" first={["kavach.toml", "src/main.kav"]} outcome="panic" />
      <P>
        Under the panic, standard error says where: <C>at src/main.kav:6</C>, the division in <C>mean</C>. The first
        call, with two scores, never made a <C>NaN</C>, so it printed as usual.
      </P>
      <P>For one run or build, <C>kavach run --watch-nan</C> or <C>kavach build --watch-nan</C> does the same without the setting.</P>
      <H2>What it watches</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The operators <C>+</C>, <C>-</C>, <C>*</C>, <C>/</C>, <C>%</C> and <C>**</C> on <C>Float</C> and <C>Float32</C>, and <C>+=</C> and the others, as your code writes them.</li>
        <li><C>std.math</C>&apos;s functions of floats. <C>math.pow(x, y)</C> shows as <C>x ** y</C>.</li>
        <li>A <C>NaN</C> that comes from outside, from a file or <C>Float.parse</C>, stops the program at the first watched operation it reaches.</li>
      </ul>
      <Files name="errors/nan-watch-math.panics" first={["src/main.kav"]} outcome="panic" />
      <P>A negative number to a fractional power has no real answer either:</P>
      <Files name="errors/nan-watch-pow.panics" first={["src/main.kav"]} outcome="panic" />
      <H2>What it doesn&apos;t watch</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The standard library&apos;s own code, which makes <C>NaN</C> on purpose, for a missing value in a <Link href="/docs/standard-library/tables/">table</Link>.</li>
        <li>Methods of lists, such as <C>sum</C>, and a <C>map</C> that runs on the GPU.</li>
        <li><C>kavach test</C>: a test may make <C>NaN</C> on purpose, to check what a function does with it.</li>
      </ul>
      <Note>
        Off, as it is by default, it costs nothing: the checks are compiled only into a program that asks for them. On,
        a loop of float arithmetic runs about seven times slower, so it is for finding a bug, not for production. It works
        the same in <C>kavach run</C>, <C>run --native</C> and <C>kavach build</C>, which names it in the program&apos;s{" "}
        <Link href="/docs/tools/the-kavach-command/build/">build line</Link>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/05-errors.md" label="spec 5.9 · watching for NaN" />
        <SrcRef file={`${TESTS}/watch-nan-names-the-operator/src/main.kav`} label="operators, tested" />
        <SrcRef file={`${TESTS}/watch-nan-names-the-math-function/src/main.kav`} label="std.math, tested" />
        <SrcRef file={`${TESTS}/nan-goes-on-without-the-watch.kav`} label="without the watch, tested" />
      </div>
    </>
  ),
};
