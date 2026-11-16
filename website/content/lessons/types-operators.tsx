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
      <H2>compare gives a sign, not a distance</H2>
      <P>
        <C>a.compare(b)</C> answers one question: which comes first? Only the sign of the answer counts, not its size:
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Below 0: <C>a</C> comes first, so <C>a &lt; b</C> is true.</li>
        <li>0: neither comes first, so <C>a &lt;= b</C> and <C>a &gt;= b</C> are both true.</li>
        <li>Above 0: <C>b</C> comes first, so <C>a &gt; b</C> is true.</li>
      </ul>
      <P>
        Numbers and text always give -1, 0 or 1. So <C>7.compare(9)</C> is -1, not 7 − 9 = -2, and{" "}
        <C>7.compare(100)</C> is -1 too: 7 comes first both times. Your own type may give any number with the right
        sign. <C>Money</C>&apos;s <C>compare</C> gives the difference in cents, -45 here, and <C>&lt;</C> reads only
        that it is below 0:
      </P>
      <Ex name="types/compare-sign" />
      <P>
        Text comes first in alphabetical order, character by character: <C>&quot;apple&quot;</C> before{" "}
        <C>&quot;pear&quot;</C>. A float&apos;s <C>compare</C> gives 0 when neither comes first, as with a{" "}
        <C>NaN</C>.
      </P>
      <P>
        <C>==</C> and <C>!=</C> are not interfaces: they compare contents for every type. So <C>a == b</C> and{" "}
        <C>compare</C> disagree only if the type&apos;s author makes them.
      </P>
      <P>
        <C>MatMul</C> gives <C>@</C>, the matrix product: see <Link href="/docs/types/matrices/">Matrices</Link>.
      </P>
      <H2>Operators in generic code</H2>
      <P>
        A type parameter with an operator&apos;s interface as its bound may use that operator. The number types meet{" "}
        <C>Add</C>, <C>Sub</C>, <C>Mul</C>, <C>Div</C>, <C>Rem</C>, <C>Neg</C> and <C>Comparable</C> as they are, and{" "}
        <C>String</C> meets <C>Comparable</C>. So one generic function serves numbers, text and your own types.
        Three functions, each asking for one operator, called with each kind of value:
      </P>
      <Ex name="types/operators-generic" />
      <P>Each function asks for only the operator it uses:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <C>total&lt;T: Add&gt;</C> adds every item to <C>zero</C>. It takes <C>zero</C> as a parameter because a
          plain <C>Add</C> has no way to make a zero: it is <C>0</C> for <C>Int</C>, <C>Float32(0.0)</C> for{" "}
          <C>Float32</C> and <C>Money(cents: 0)</C> for <C>Money</C>. Numbers add with their own <C>+</C>;{" "}
          <C>Money</C> adds with its <C>add</C> method.
        </li>
        <li>
          <C>flip&lt;T: Neg&gt;</C> uses unary <C>-</C>: a number turns negative, and <C>Money</C> calls its{" "}
          <C>neg</C> method.
        </li>
        <li>
          <C>largest&lt;T: Comparable&gt;</C> keeps the item that comes last in order, using <C>&gt;</C>. Numbers
          compare by size, text in alphabetical order (so <C>&quot;zebra&quot;</C> wins), and <C>Money</C> by its{" "}
          <C>compare</C> method. An empty list has no largest item, so it gives <C>none</C>.
        </li>
        <li>
          Each type a function is called with gets its own compiled copy, so for a number <C>a + b</C> is one machine
          instruction, as if you had written the function for that type.
        </li>
        <li>
          A number&apos;s methods are the operators, so a number meets these bounds: <C>7.add(2)</C>, the last line,
          is <C>9</C>, the same as <C>7 + 2</C>. Its <C>compare</C> is its ordering: <C>7.compare(9)</C> is -1, as 7
          comes first (see above).
        </li>
        <li>For any number at all, <C>T: Number</C> gives every operator at once: see <Link href="/docs/types/generics/">Generics</Link>.</li>
      </ul>
      <P>A type given for such a <C>T</C> must have the operator. <C>String</C> has no <C>Add</C>:</P>
      <Ex name="types/operator-bound-string.fails" />
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
        <SrcRef file={`${TESTS}/operators-in-generic-code.kav`} label="generic code, tested" />
        <SrcRef file="compiler/checker/src/user_ops.rs" label="user_ops.rs" />
      </div>
    </>
  ),
  tryIt: {
    task: <>Give <C>Money</C> a <C>Sub</C>, and print whether <C>Money(cents: 500) - Money(cents: 120)</C> is above <C>Money(cents: 300)</C>.</>,
    answer: <P>Add <C>Sub</C> to the type&apos;s list and write <C>fn sub(self, other: Money) -&gt; Money</C>, giving <C>Money(cents: self.cents - other.cents)</C>. The difference is 380 cents, so it prints <C>true</C>.</P>,
  },
};
