import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// Whole numbers with numbers in their types (spec 2.1, decision 0137):
// Int<bits: n>, UInt<bits: n> and Int<range: a..=b>.

const SPEC = "docs/spec/02-types/01-built-in-types.md";
const CHECK = "compiler/checker/src";
const TESTS = "tests/conformance/02-types";

export const bitsDoc: LessonDoc = {
  lead: "Int<bits: n> and UInt<bits: n> hold a whole number of exactly n bits, for hardware registers, chip design and file formats; Int8 is Int<bits: 8>.",
  simple: <>Some numbers live in a fixed number of switches, such as a 12-bit counter on a chip. <C>UInt&lt;bits: 12&gt;</C> says exactly that: 12 switches, so 0 to 4095. Going past the top stops the program, unless you ask it to roll over as the chip does.</>,
  minutes: 5,
  before: ["basics/data-types/sized-integers"],
  body: (
    <>
      <Ex name="datatypes/bits" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Int&lt;bits: n&gt;</C> is signed: <C>Int&lt;bits: 24&gt;</C> holds -8,388,608 to 8,388,607.</li>
        <li><C>UInt&lt;bits: n&gt;</C> is never negative: <C>UInt&lt;bits: 12&gt;</C> holds 0 to 4095.</li>
        <li>
          The sized names are short spellings of these, so each pair is one type: <C>Int8</C> is{" "}
          <C>Int&lt;bits: 8&gt;</C>, <C>UInt64</C> is <C>UInt&lt;bits: 64&gt;</C>, and <C>Int</C> is{" "}
          <C>Int&lt;bits: 64&gt;</C>.
        </li>
        <li>The width is a number written in the type, or a constant known while compiling.</li>
      </ul>
      <P>A number written in the code that doesn&apos;t fit is refused before the program runs, with a type that holds it:</P>
      <Ex name="datatypes/bits-too-big.fails" />
      <P><C>UInt</C> alone has no width, so it is refused too, with the two ways to write one:</P>
      <Ex name="datatypes/bits-uint.fails" />
      <H2>Arithmetic at the width</H2>
      <P>
        <C>+ - * / % **</C> and the comparisons work as on an <C>Int</C>. Both sides have the same type, and a result
        that doesn&apos;t fit stops the program, as an <C>Int</C>&apos;s{" "}
        <Link href="/docs/basics/data-types/sized-integers/">overflow</Link> does:
      </P>
      <Ex name="datatypes/bits-overflow.panics" />
      <P>
        A register rolls over instead. Ask for that by name: <C>wrapping_add</C>, <C>wrapping_sub</C> and{" "}
        <C>wrapping_mul</C> wrap at the type&apos;s width, and <C>checked_add</C> gives <C>none</C> when the sum
        doesn&apos;t fit:
      </P>
      <Ex name="datatypes/bits-wrap" />
      <H2>The bit operators</H2>
      <P>
        <C>&amp; | ^</C> and the shifts work on the number&apos;s bits, and <C>~</C> flips all <C>n</C> of them, not
        64: <C>~0xABC</C> in 12 bits is <C>0x543</C>, which is 1347.
      </P>
      <Ex name="datatypes/bits-ops" />
      <P>
        A shift takes 0 to <C>n - 1</C>. A shift to the left that pushes a 1 out of the top stops the program, as any
        overflow does: <C>0xABC &lt;&lt; 4</C> in 12 bits panics, so the sample keeps the low 8 bits first.
      </P>
      <H2>Converting</H2>
      <P>
        <C>UInt&lt;bits: 12&gt;(x)</C> converts any number, and panics when it doesn&apos;t fit, as{" "}
        <C>Int8(300)</C> does. Under <C>catch</C> or <C>try</C>, it fails with a <C>SizeError</C> instead, with the
        same text. This holds for <C>Int8</C> … <C>UInt64</C> too, as they are <C>bits</C> types. A float is cut to
        its whole part first.
      </P>
      <Ex name="datatypes/bits-convert" />
      <H2>In lists, records and JSON</H2>
      <P>They print, sort, compare, and go in lists, dicts and records. JSON gets the plain number:</P>
      <Ex name="datatypes/bits-record" />
      <P>
        The spec allows widths up to 65,535 bits, but only 64 are built so far. A wider one is refused, and the
        message names <Link href="/docs/basics/data-types/bigint/"><C>BigInt</C></Link>:
      </P>
      <Ex name="datatypes/bits-wide.fails" />
      <Note>
        Not built yet: widths over 64 bits; keeping a value in the smallest of 1, 2, 4 or 8 bytes (today each takes 8, as an{" "}
        <C>Int</C>), and, in a native build, <C>json.decode</C> into a type holding one of these numbers, and a
        generic function given one. The interpreter runs both.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.1 · Int<bits: n>" />
        <SrcRef file="docs/decisions/0137-numbers-in-types.md" label="decision 0137" />
        <SrcRef file={`${CHECK}/int_numbers.rs`} line={44} symbol="fn bits_type" />
        <SrcRef file="compiler/interp/src/ops.rs" line={215} label="overflow at the width (interpreter)" />
        <SrcRef file="runtime/native/src/rt.rs" line={305} label="overflow at the width (native)" />
        <SrcRef file="compiler/interp/src/conversions.rs" line={66} label="conversions" />
        <SrcRef file={`${TESTS}/int8-is-int-of-8-bits.kav`} label="Int8 is Int<bits: 8>, tested" />
        <SrcRef file={`${TESTS}/a-uint-of-12-bits-panics-past-4095.kav`} label="overflow, tested" />
        <SrcRef file={`${TESTS}/a-uint-of-12-bits-wraps-when-asked.kav`} label="wrapping, tested" />
        <SrcRef file={`${TESTS}/bit-operators-work-at-the-width.kav`} label="bit operators, tested" />
        <SrcRef file={`${TESTS}/a-shift-past-the-width-panics.kav`} label="shifts, tested" />
        <SrcRef file={`${TESTS}/try-into-bits-fails-with-a-size-error.kav`} label="SizeError, tested" />
        <SrcRef file={`${TESTS}/bits-numbers-go-in-records-lists-and-json.kav`} label="records and JSON, tested" />
      </div>
    </>
  ),
};

export const rangeDoc: LessonDoc = {
  lead: "Int<range: a..=b> is a whole number that is always in the range, such as a port from 1 to 65535; a value outside it never gets in.",
  simple: <>Some numbers only make sense between two ends: a month is 1 to 12, a port is 1 to 65535. A range type writes those ends down once, and Kavach checks every value that goes in, so a 0 or a 70000 can&apos;t sneak through.</>,
  minutes: 4,
  before: ["basics/data-types/any-bit-width"],
  body: (
    <>
      <Ex name="datatypes/range" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <C>Int&lt;range: a..=b&gt;</C> holds <C>a</C> to <C>b</C>. <C>a..b</C> leaves <C>b</C> out, as a{" "}
          <Link href="/docs/basics/for-loop/over-a-range/">range</Link> does: <C>Int&lt;range: 0..10&gt;</C> is the type{" "}
          <C>Int&lt;range: 0..=9&gt;</C>, and messages show it that way.
        </li>
        <li>Both ends are known while compiling. An empty range is a compile error.</li>
        <li>Arithmetic on two values of one range type gives that type. A number on the right of an operator or a comparison need only be an <C>Int</C>, since the result is what gets checked: <C>p - 1</C> is fine though 1 is not in 10 to 20.</li>
        <li><C>checked_add</C> gives <C>none</C> when the result is out of the range. <C>Int(p)</C> always converts back.</li>
      </ul>
      <P>A number written in the code must be in the range:</P>
      <Ex name="datatypes/range-literal.fails" />
      <P>A result out of the range stops the program, as overflow does:</P>
      <Ex name="datatypes/range-overflow.panics" />
      <H2>Converting into a range</H2>
      <P>
        <C>Int&lt;range: 1..=65535&gt;(n)</C> checks <C>n</C>. Under <C>try</C> or <C>catch</C> it fails with a{" "}
        <C>SizeError</C>, so a function can hand the problem to its caller:
      </P>
      <Ex name="datatypes/range-try" />
      <P>Without <C>try</C> or <C>catch</C>, a number out of the range panics:</P>
      <Ex name="datatypes/range-half-open.panics" />
      <H2>What a range doesn&apos;t do</H2>
      <P>Wrapping round has no meaning for a range, so <C>wrapping_add</C> is refused, with what to use:</P>
      <Ex name="datatypes/range-wrap.fails" />
      <P>
        The bit operators <C>&amp; | ^ &lt;&lt; &gt;&gt; ~</C> work on bits, not on values in a range. Convert first,
        or use an <Link href="/docs/basics/data-types/any-bit-width/"><C>Int&lt;bits: n&gt;</C></Link>:
      </P>
      <Ex name="datatypes/range-bits.fails" />
      <P>
        Two ranges are two types, and a range is not a <C>bits</C> type: <C>Int&lt;range: 0..=255&gt;</C> is not{" "}
        <C>UInt8</C>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.1 · Int<range: a..=b>" />
        <SrcRef file="docs/decisions/0137-numbers-in-types.md" label="decision 0137" />
        <SrcRef file={`${CHECK}/constants.rs`} line={127} label="a literal outside the range" />
        <SrcRef file={`${CHECK}/int_numbers.rs`} line={59} symbol="fn range_hint" />
        <SrcRef file={`${CHECK}/int_numbers.rs`} line={259} label="bit operators refused" />
        <SrcRef file={`${TESTS}/a-range-keeps-its-range-through-arithmetic.kav`} label="arithmetic, tested" />
        <SrcRef file={`${TESTS}/a-literal-beside-a-range-need-only-be-an-int.kav`} label="literals beside it, tested" />
        <SrcRef file={`${TESTS}/a-range-up-to-its-end-leaves-the-end-out.kav`} label="a..b, tested" />
        <SrcRef file={`${TESTS}/checked-add-on-a-range-gives-none.kav`} label="checked_add, tested" />
        <SrcRef file={`${TESTS}/try-into-a-range-fails-with-a-size-error.kav`} label="SizeError, tested" />
        <SrcRef file={`${TESTS}/wrapping-on-a-range-is-an-error.kav`} label="no wrapping, tested" />
        <SrcRef file={`${TESTS}/bit-operators-on-a-range-are-errors.kav`} label="no bit operators, tested" />
      </div>
    </>
  ),
};
