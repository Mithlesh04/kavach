import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { WhatIsTested } from "./tested-table";

// ---------- Evaluation Order ----------
//
// Spec 3.10: everything is worked out once, left to right, as written,
// in both the interpreter and native code.

const TESTS = "tests/conformance/03-functions";

export const orderDoc: LessonDoc = {
  lead: "Everything is worked out once, left to right, as it is written: the same in kavach run and in a built program.",
  simple: <>Kavach reads your code the way you do: from left to right. If two calls print something, the one written first prints first, every time.</>,
  minutes: 6,
  before: ["functions/functions", "functions/named-arguments"],
  body: (
    <>
      <P>
        Order matters when the parts of a line do something: print, read a file, draw a random number. Here{" "}
        <C>say</C> prints its number and gives it back, so the output shows the order each part ran in:
      </P>
      <Ex name="functions/order" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>say(1).wrapping_add(say(2))</C>: the value a method is called on comes before its arguments.</li>
        <li><C>add(b: say(3), a: say(4))</C>: arguments run as written, named ones too, whatever order the parameters are declared in.</li>
        <li><C>say(5) + say(6)</C>: an operator&apos;s left side before its right side.</li>
        <li><C>Point(y: say(7), x: say(8))</C>: a record&apos;s fields as written, not in the order the type declares them.</li>
        <li><C>grid[say(1)][say(0)] += say(9)</C>: the place&apos;s keys first, then the value.</li>
      </ul>
      <P>
        The order is a rule of the language, not a choice of the compiler, so <C>kavach run</C> and a program built
        with <C>kavach build</C> print the same lines.
      </P>
      <H2>and, or and ??</H2>
      <P>
        Their left side always runs first. The right side runs only when it is needed: <C>and</C> stops at a{" "}
        <C>false</C>, <C>or</C> at a <C>true</C>, and <C>??</C> at a value that is not <C>none</C>:
      </P>
      <Ex name="functions/order-short" />
      <P>
        A chain of comparisons, <C>0 &lt;= i &lt; n</C>, works out each value once and stops at the first part that is
        false (<Link href="/docs/basics/operators/comparison/">comparison operators</Link>).
      </P>
      <H2>Values written out</H2>
      <P>
        The items of a list or tuple, a dict&apos;s entries (each key before its value) and the parts of a text
        template run in the order written:
      </P>
      <Ex name="functions/order-written" />
      <P>
        A dict key in brackets, <C>{"{[say(\"ann\")]: …}"}</C>, is the value of what is inside (
        <Link href="/docs/collections/dicts/">dicts</Link>).
      </P>
      <H2>Defaults, and a function worked out first</H2>
      <P>
        Default values run after the arguments written, in the parameters&apos; order. When the function itself comes
        from an expression, as in <C>pick(1)(say(5))</C>, that expression runs before the arguments:
      </P>
      <Ex name="functions/order-defaults" />
      <H2>Changing a place</H2>
      <P>
        <C>place = value</C> works out the place&apos;s keys, left to right, then the value, then sets the place.{" "}
        <C>place += value</C> reads and sets the same place, but each key still runs only once. A method that changes
        its value in place, as <C>push</C> does, works out the keys once, before its arguments:
      </P>
      <Ex name="functions/order-places" />
      <H2>The two exceptions</H2>
      <P>
        <strong className="text-ink">Tuple assignment</strong> works out its whole right side first, then sets each
        place, left to right. That is what lets <C>(a, b) = (b, a)</C> swap two values (
        <Link href="/docs/basics/variables/tuple-assignment/">tuple assignment</Link>).
      </P>
      <P>
        <strong className="text-ink">An inout argument</strong> has its keys worked out twice: once with the
        call&apos;s other arguments, and again when its new value goes back after the call (
        <Link href="/docs/functions/inout-parameters/">inout parameters</Link>). So <C>pick</C> runs twice here:
      </P>
      <Ex name="functions/order-inout" />
      <P>
        This one is planned to change: the spec lists it as an open question, and the plan is to work the keys out
        once, with the arguments, as for every other place.
      </P>
      <P>
        <C>spawn</C> works out a call&apos;s value and arguments the same way, before the task starts (
        <Link href="/docs/concurrency/tasks/">tasks</Link>).
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/03-functions.md" label="spec 3.10 · the order things are worked out in" />
        <SrcRef file="compiler/interp/src/receivers.rs" line={59} symbol="fn receiver_first" />
        <SrcRef file="compiler/interp/src/places.rs" line={127} symbol="fn place" />
        <SrcRef file="compiler/kir/src/lower_order.rs" line={42} symbol="fn in_written_order" />
        <SrcRef file="compiler/kir/src/lower_order.rs" line={92} symbol="fn hold_keys" />
        <SrcRef file={`${TESTS}/arguments-are-worked-out-as-written.kav`} label="arguments, tested" />
        <SrcRef file={`${TESTS}/operands-are-worked-out-left-to-right.kav`} label="operands, tested" />
        <SrcRef file={`${TESTS}/a-changed-places-keys-are-worked-out-once.kav`} label="keys once, tested" />
      </div>
      <WhatIsTested number="3.10" />
    </>
  ),
  tryIt: {
    task: <>What does <C>print(say(1) - say(2) * say(3))</C> print, with <C>say</C> as on this page?</>,
    answer: (
      <P>
        <C>1</C>, <C>2</C> and <C>3</C>, then <C>-5</C>. <C>*</C> binds tighter, so it is done first, but each
        part is still worked out left to right.
      </P>
    ),
  },
};
