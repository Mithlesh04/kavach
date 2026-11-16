import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- Generics ----------

export const genericsDoc: LessonDoc = {
  lead: "A generic function or type works for any type: fn first<T>(items: [T]) -> T? works on a list of anything.",
  simple: <>A function that gives the first item of a list doesn&apos;t care what the list holds: numbers, words, anything. A generic lets you write it once, for every kind of thing.</>,
  minutes: 5,
  before: ["functions/functions", "types/records"],
  body: (
    <>
      <Ex name="types/generics-fn" />
      <P>
        <C>&lt;T&gt;</C> is a placeholder for a type. Each call fills it in: <C>first([3, 1, 2])</C> uses{" "}
        <C>Int</C>, <C>first([&quot;x&quot;, &quot;y&quot;])</C> uses <C>String</C>. Types like{" "}
        <C>Pair&lt;A, B&gt;</C> work the same way. A method can have its own placeholders too:
      </P>
      <Ex name="types/generics-methods" />
      <H2>Type arguments written where a value is made</H2>
      <P>
        A generic type&apos;s arguments are usually worked out from its fields and from where the value goes. When no
        field shows them, write them where the value is made, or give the variable its type. Inside the type&apos;s
        methods, <C>I</C> is then the type the value was made with:
      </P>
      <Ex name="types/generics-construction" />
      <P>The count must match the type&apos;s parameters:</P>
      <Ex name="types/generics-arity.fails" />
      <H2>Math in generic code</H2>
      <P>
        A plain <C>T</C> could be text, so <C>+</C> on it is refused. An operator&apos;s bound, as <C>T: Add</C>, gives
        one operator (see <Link href="/docs/types/operators-for-your-types/">Operators for Your Types</Link>). Two
        built-in bounds say it is a number.{" "}
        <C>T: Number</C> is met by every number type; <C>T: Floating</C> only by <C>Float</C> and <C>Float32</C>, and
        lets the code call <C>std.math</C> on a <C>T</C>:
      </P>
      <Ex name="types/number-bounds" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A number literal in such code is a <C>T</C>, and <C>T(0)</C> turns any number into a <C>T</C>.</li>
        <li><C>math.sqrt</C> of a <C>Float32</C> gives a <C>Float32</C>, so <C>length</C> gives back the type it was given.</li>
        <li>Each type gets its own compiled copy, so <C>total</C> of <C>Float32</C>s runs as a loop written for <C>Float32</C> would.</li>
        <li>The bound is <C>Floating</C>, not <C>Float</C>, as <C>Float</C> is a type.</li>
      </ul>
      <P>A number with a fraction is a <C>T</C> only when <C>T</C> is <C>Floating</C>; 0.5 of an <C>Int</C> isn&apos;t one:</P>
      <Ex name="types/fraction-needs-floating.fails" />
      <P>And a type that is no number doesn&apos;t meet the bound:</P>
      <Ex name="types/number-bound-string.fails" />
      <H2>A bound on one method</H2>
      <P>
        A generic type&apos;s method may add a bound to the type&apos;s parameter with <C>where</C>, after its result
        type. That method is then there only for the types that meet the bound; the type&apos;s other methods are
        there for all of them:
      </P>
      <Ex name="types/method-bound" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>where</C> bound is <C>Number</C> or <C>Floating</C>, and names the type&apos;s own parameters.</li>
        <li>In the method&apos;s body the parameter has the bound, so <C>2.0</C> and <C>std.math</C> may be used on a <C>T</C>.</li>
        <li>A literal with no type of its own becomes a <C>Float</C> there: <C>Pair(a: 1, b: 2).mean()</C> is <C>1.5</C>.</li>
        <li>A function writes its bounds in <C>&lt;T: …&gt;</C>. <C>where</C> is a keyword only on a method&apos;s line; elsewhere it may be a name.</li>
      </ul>
      <P>Calling the method on a type that doesn&apos;t meet the bound is an error that names the bound:</P>
      <Ex name="types/method-bound.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/09-generics.md" label="spec 2.9 · generics" />
        <SrcRef file="tests/conformance/02-types/generic-type-methods-know-their-type-arguments.kav" label="tested" />
        <SrcRef file="tests/conformance/02-types/number-bounds-do-arithmetic.kav" label="number bounds, tested" />
        <SrcRef file="docs/decisions/0115-generic-math-and-std-vec.md" label="decision 0115" />
        <SrcRef file="tests/conformance/02-types/a-method-can-add-a-bound.kav" label="where, tested" />
        <SrcRef file="docs/decisions/0119-bounds-on-one-method.md" label="decision 0119" />
      </div>
    </>
  ),
  deeper: (
    <P>
      A generic function is compiled once for each type it is used with, so it runs as fast as if you had written a
      copy for that type yourself. To hold values of different types together in one list instead, use an{" "}
      <Link href="/docs/types/interface-values/">interface value</Link>.
    </P>
  ),
};
