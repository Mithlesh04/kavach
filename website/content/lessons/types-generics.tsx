import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { fileTables } from "@/lib/spec-tables";
import { Ex, H2, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Generics ----------

const SPEC = "docs/spec/02-types/09-generics.md";
const TESTS = "tests/conformance/02-types";

/** The spec's table of the built-in types that take numbers, only those built (lib/built-spec.ts). */
const numberTypes = () => {
  const t = fileTables(SPEC).find((x) => x.header[0] === "Written" && x.header[1] === "Means");
  if (!t) throw new Error(`${SPEC}: no table of types with numbers`);
  return t;
};

export const genericsDoc: LessonDoc = {
  lead: "A generic function or type works for any type: fn first<T>(items: [T]) -> T? works on a list of anything. A type may take numbers too, as Grid<Float, width: 2, height: 3>.",
  simple: <>A function that gives the first item of a list doesn&apos;t care what the list holds: numbers, words, anything. A generic lets you write it once, for every kind of thing.</>,
  minutes: 9,
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
      <P>When nothing says what a type argument is, the compiler asks for it:</P>
      <Ex name="types/generics-unfixed.fails" />
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
      <P><C>Float</C> as a bound is a compile error that names the right one:</P>
      <Ex name="types/float-bound.fails" />
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
      <H2>Numbers in types</H2>
      <P>
        A generic type may take numbers as well as types: a grid&apos;s width, a buffer&apos;s size. Types are
        positional and numbers are named, as named arguments are in a call. The types come first, in order; then
        each number, with its name, in any order:
      </P>
      <Ex name="types/number-params" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Inside the type, a number parameter is a constant <C>Int</C>: <C>width * height</C> above.</li>
        <li>The same names with the same numbers are the same type, whatever their order.</li>
        <li>A number parameter may have a default, as a function&apos;s parameter may:</li>
      </ul>
      <Ex name="types/number-default" />
      <P>
        A function may take numbers too. They are worked out from its arguments, or from the type wanted, as type
        arguments are; when nothing shows them, they are written at the call:
      </P>
      <Ex name="types/number-fn" />
      <P>Both lists of <C>dot</C> have length <C>n</C>, so lists of two lengths are refused:</P>
      <Ex name="types/number-two-values.fails" />
      <P>The built-in types that take numbers, and where each is explained:</P>
      <Table header={numberTypes().header} rows={numberTypes().rows} />
      <H2>Numbers that are refused</H2>
      <P>Every number without a default is given, with its name:</P>
      <Ex name="types/number-missing.fails" />
      <Ex name="types/number-unnamed.fails" />
      <P>A number in a type is known while compiling: a literal, a constant, or a number parameter. One known only while running is refused:</P>
      <Ex name="types/number-running.fails" />
      <Ex name="types/number-const" />
      <P>The same type with other numbers is another type:</P>
      <Ex name="types/number-two-types.fails" />
      <H2>Numbers in variables and operators</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          A <C>let</C> with no written type drops the collection&apos;s own number, as its value may change; a{" "}
          <C>const</C> keeps it, and so does a <C>let</C> whose type is written. A <C>max</C> limit stays with the
          value while running either way.
        </li>
        <li>Comparisons and arithmetic don&apos;t look at the number: <C>xs + ys</C> is a plain list.</li>
      </ul>
      <Ex name="types/number-let" />
      <P>
        Inside code with a number parameter, a literal can&apos;t be checked against <C>len: n</C>, as <C>n</C> is
        known only for each call. <C>List&lt;T, len: n&gt;.filled(v)</C> makes the list instead:
      </P>
      <Ex name="types/number-literal-generic.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0137-numbers-in-types.md" label="decision 0137" />
        <SrcRef file="compiler/checker/src/sizes.rs" label="the checker's numbers in types" />
        <SrcRef file={`${TESTS}/a-type-takes-number-parameters.kav`} label="numbers in a type, tested" />
        <SrcRef file={`${TESTS}/number-arguments-are-named-in-any-order.kav`} label="any order, tested" />
        <SrcRef file={`${TESTS}/a-number-parameter-may-have-a-default.kav`} label="defaults, tested" />
        <SrcRef file={`${TESTS}/a-functions-number-parameter-is-worked-out.kav`} label="a function's, tested" />
        <SrcRef file={`${TESTS}/a-missing-number-argument-is-an-error.kav`} label="missing, tested" />
        <SrcRef file={`${TESTS}/two-numbers-make-two-types.kav`} label="two types, tested" />
      </div>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.9 · generics" />
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
