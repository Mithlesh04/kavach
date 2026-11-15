import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { fileTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Operators for Your Types ----------
//
// Spec 2.16: a type gets +, -, *, /, %, unary - and the ordering
// operators by implementing Add, Sub, Mul, Div, Rem, Neg or Comparable.

const SPEC = "docs/spec/02-types/16-operators.md";
const TESTS = "tests/conformance/02-types";

/** The spec's table of interfaces, operators and methods. */
const interfaces = () => {
  const t = fileTables(SPEC).find((x) => x.header[0] === "Interface");
  if (!t) throw new Error(`${SPEC}: no Interface table`);
  return t;
};

export const ownOperatorsDoc: LessonDoc = {
  lead: "A type gets +, -, *, /, % and < by implementing Add, Sub, Mul, Div, Rem or Comparable. Then vectors, money and units read like numbers.",
  simple: <>Numbers can be added with <C>+</C>. Your own type can be too, once it says how: you write an <C>add</C> method, and <C>a + b</C> calls it. Nothing happens by magic, and a type without the method has no <C>+</C>.</>,
  minutes: 6,
  before: ["types/interfaces", "basics/operators"],
  body: (
    <>
      <Ex name="types/operators" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>type Vec2: Add, Neg, Mul&lt;Float&gt;</C> says which operators the type has, as with any <Link href="/docs/types/interfaces/">interface</Link>.</li>
        <li><C>a + b</C> calls <C>a.add(b)</C>, <C>-b</C> calls <C>b.neg()</C>, and <C>a * 2.0</C> calls <C>a.mul(2.0)</C>.</li>
        <li><C>c += a</C> is <C>c = c + a</C>, as for numbers, so it needs <C>Add</C> and a <C>let</C>.</li>
      </ul>
      <H2>Every operator and its method</H2>
      <Table header={interfaces().header} rows={interfaces().rows} />
      <H2>The other operand</H2>
      <P>
        <C>Add</C> alone takes the same type and gives it back. <C>Mul&lt;Float&gt;</C> takes a <C>Float</C> instead.
        With a second type argument, the result is another type: <C>Mul&lt;Meters, Area&gt;</C> takes <C>Meters</C>{" "}
        and gives an <C>Area</C>:
      </P>
      <Ex name="types/operators-other-type" />
      <P>
        A type implements each interface once, since its method has one name and Kavach has{" "}
        <Link href="/docs/functions/functions/declaring/">no overloading</Link>. So <C>Vec2</C> with{" "}
        <C>Mul&lt;Float&gt;</C> has <C>v * 2.0</C> but not <C>v * v</C>.
      </P>
      <H2>Ordering with Comparable</H2>
      <P>
        <C>Comparable</C> gives <C>&lt;</C>, <C>&lt;=</C>, <C>&gt;</C> and <C>&gt;=</C> from one method,{" "}
        <C>compare</C>: below 0 when <C>self</C> comes first, 0 when neither does, above 0 when the other does:
      </P>
      <Ex name="types/operators-comparable" />
      <P>
        <C>==</C> and <C>!=</C> are not interfaces: they compare contents for every type. So <C>a == b</C> and{" "}
        <C>compare</C> disagree only if the type&apos;s author makes them.
      </P>
      <P>
        <C>MatMul</C> gives <C>@</C>, the matrix product: see <Link href="/docs/types/matrices/">Matrices</Link>.
      </P>
      <H2>What is refused</H2>
      <P>Your type goes on the left. <C>2.0 * v</C> is an error that says to write <C>v * 2.0</C>:</P>
      <Ex name="types/operators-left.fails" />
      <P>
        Operator methods are pure: no <C>uses</C> and no <C>!</C>. So <C>a + b</C> never touches the outside world and
        never fails, except by a panic, as <C>Int</C> overflow does:
      </P>
      <Ex name="types/operators-pure.fails" />
      <P>A type without the interface has no operator:</P>
      <Ex name="types/operators-missing.fails" />
      <P>Built-in types keep their own operators:</P>
      <Ex name="types/operators-built-in.fails" />
      <Note>
        Other languages call this operator overloading. In Kavach the operator is a promise the type makes, like any
        interface, so a reader sees it in the type&apos;s first line.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.16 · operators" />
        <SrcRef file="docs/decisions/0109-operators-through-interfaces.md" label="decision 0109" />
        <SrcRef file={`${TESTS}/operators-call-the-types-own-methods.kav`} label="tested" />
        <SrcRef file={`${TESTS}/an-operator-can-give-another-type.kav`} label="another type, tested" />
        <SrcRef file={`${TESTS}/comparable-gives-ordering-operators.kav`} label="Comparable, tested" />
        <SrcRef file="compiler/checker/src/user_ops.rs" label="user_ops.rs" />
      </div>
    </>
  ),
  tryIt: {
    task: <>Give <C>Money</C> a <C>Sub</C>, and print whether <C>Money(cents: 500) - Money(cents: 120)</C> is above <C>Money(cents: 300)</C>.</>,
    answer: <P>Add <C>Sub</C> to the type&apos;s list and write <C>fn sub(self, other: Money) -&gt; Money</C>, giving <C>Money(cents: self.cents - other.cents)</C>. The difference is 380 cents, so it prints <C>true</C>.</P>,
  },
};
