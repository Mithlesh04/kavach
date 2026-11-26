import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { TENSOR_SPEC, TENSOR_SRC, TENSOR_TESTS, tensorTable } from "./stdlib-tensor";

// ---------- Tensors: making them, and indexing ----------
//
// Spec 14.33's "Making tensors" and "Shape, items and indexing", and 2.19's
// many dimensions.

export const tensorMakeDoc: LessonDoc = {
  lead: "tensor.of, of_rows, zeros, ones, full and eye make a tensor from numbers; arange and linspace count; random and normal draw, the same under one seed on every machine.",
  simple: <>To start working with a grid of numbers you either write the numbers out, fill a shape with one number, or count along a line. Random ones come from the same generator as the rest of Kavach, so a seed gives the same numbers every time.</>,
  minutes: 5,
  before: ["standard-library/tensors"],
  body: (
    <>
      <Ex name="tensor/make" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>zeros</C>, <C>ones</C> and <C>eye</C> name the number type, as nothing else says it; <C>full</C>, <C>of</C> and <C>of_rows</C> take it from their numbers.</li>
        <li>A shape&apos;s sizes are 0 or more. <C>[]</C> is a shape too: one number and no dimensions.</li>
        <li><C>of_rows</C> needs every row as long; rows of different lengths are a panic naming both lengths.</li>
      </ul>
      <Ex name="tensor/rows.panics" />
      <H2>Counting</H2>
      <Ex name="tensor/ranges" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>arange</C> counts as <C>start..end</C> does: <C>end</C> is left out. Each number is worked out as <C>start + i * step</C>, so no rounding builds up. A step of 0 is a panic.</li>
        <li><C>linspace</C> gives <C>count</C> numbers with both ends included, and the last is exactly <C>end</C>.</li>
      </ul>
      <H2>Random numbers</H2>
      <P>
        <C>random</C>, <C>normal</C> and <C>random_int</C> draw through{" "}
        <Link href="/docs/standard-library/random-numbers/">std.random</Link>, in order. So{" "}
        <C>random.seeded(s) {"{ … }"}</C> gives the same tensor on every machine, and under <C>kavach test</C> the{" "}
        <C>Random</C> fake gives repeatable ones:
      </P>
      <Ex name="tensor/random" />
      <P>
        <C>random_int</C> draws each number exactly as <C>random.int</C> draws it, so under one seed a tensor of 10
        gives the same numbers as 10 calls of <C>random.int</C>:
      </P>
      <Ex name="tensor/random-int" />
      <P>
        One call draws at most 16,777,216 (2<sup>24</sup>) numbers, a 4,096 × 4,096 matrix. Asking for more breaks
        the function&apos;s <Link href="/docs/contracts/">contract</Link>:
      </P>
      <Ex name="tensor/random-int-bound.panics" />
      <P>The bound lets the tests <C>kavach test</C> makes from that contract end in time: they skip shapes it rules out.</P>
      <P>They need <C>uses Random</C>, as every random number does. <C>normal</C> uses the Box–Muller method, with <C>std.math</C>&apos;s functions, which give the same bits everywhere.</P>
      <H2>Converting</H2>
      <P><C>Tensor&lt;U&gt;(t)</C> converts each number to <C>U</C>, as <C>U(x)</C> converts one:</P>
      <Ex name="tensor/convert" />
      <H2>Every call</H2>
      {tensorTable("Making tensors")}
      <Note>
        Not built yet: the <C>dims:</C> in these result types. Today each gives a plain <C>Tensor&lt;T&gt;</C>, and a
        program writes <C>Tensor&lt;T&gt;</C> too: <C>fn f(t: Tensor&lt;Float&gt;)</C>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · making tensors" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={239} symbol="fn of" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={268} symbol="fn arange" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={302} symbol="fn random" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-arange-counts-by-its-step.kav`} label="arange, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-linspace-ends-exactly-at-end.kav`} label="linspace, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-eye-has-ones-on-the-diagonal.kav`} label="eye, tested" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={335} symbol="fn random_int" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-random-repeats-under-a-seed.kav`} label="random, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-random-int-draws-as-random-int-does.kav`} label="random_int as random.int, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-random-int-draws-many-at-once.kav`} label="many at once, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-random-int-over-its-bound-panics.kav`} label="the bound, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-conversion-converts-each-number.kav`} label="converting, tested" />
      </div>
    </>
  ),
};

export const tensorIndexDoc: LessonDoc = {
  lead: "t[i, j] reads one number, with one index per dimension; t[0:2, :, #1] is a slice of each dimension, as NumPy writes them. #k counts from the end of its own dimension.",
  simple: <>A list needs one number to find an item; a grid needs two, the row and the column. A tensor takes one for each of its dimensions, separated by commas.</>,
  minutes: 5,
  before: ["standard-library/tensors", "collections/indexing-and-slicing"],
  body: (
    <>
      <Ex name="tensor/index" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each place between the commas is a number, <C>#k</C> or a slice, as for a list (<Link href="/docs/collections/indexing-and-slicing/">Indexing and Slicing</Link>).</li>
        <li><C>get(at)</C> gives <C>none</C> where <C>t[…]</C> would panic, as a list&apos;s <C>get</C> does.</li>
        <li><C>item()</C> gives the one number of a tensor that holds exactly one, of any shape.</li>
      </ul>
      <P>An index out of range is a panic naming it and the shape:</P>
      <Ex name="tensor/out-of-range.panics" />
      <P>
        A key of numbers only needs one for each dimension. NumPy gives row 1 for <C>t[1]</C> of a matrix; here it
        is a panic that says to write the row as a slice:
      </P>
      <Ex name="tensor/one-number.panics" />
      <P><C>item()</C> of a tensor of more than one number is a panic too:</P>
      <Ex name="tensor/item.panics" />
      <H2>Slices</H2>
      <P>
        A key with a slice gives a new tensor. A slice keeps its dimension, and a number drops it. Fewer places than
        dimensions take the rest whole:
      </P>
      <Ex name="tensor/slice" />
      <H2>Setting a number</H2>
      <P><C>t[i, j] = v</C> changes one number of a <C>let</C> tensor. The tensor is a value, so its copies keep their numbers:</P>
      <Ex name="tensor/set" />
      <H2>Setting a slice</H2>
      <P>
        <C>t[0, :] = u</C> sets every number the slice picks. <C>u</C> is a tensor broadcast to the slice&apos;s shape,
        or one number that each of them takes:
      </P>
      <Ex name="tensor/set-slice" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>u</C> is read whole before any number changes, so <C>g[1:, :] = g[:#1, :]</C> moves rows down, as NumPy does.</li>
        <li>The slice is clamped first, as every slice is.</li>
      </ul>
      <P>A shape that doesn&apos;t broadcast to the slice&apos;s is a panic naming both:</P>
      <Ex name="tensor/set-slice-shape.panics" />
      <H2>Every call</H2>
      {tensorTable("Shape, items and indexing")}
      <P>
        A tensor is indexed through the same interfaces as a program&apos;s own types, with a key of any length: a
        list of <C>Part</C>s. See <Link href="/docs/types/operators-for-your-types/">Operators for Your Types</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · indexing" />
        <SrcRef file="docs/spec/02-types/19-indexing-and-slicing.md" label="spec 2.19 · many dimensions" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={57} symbol="fn index" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={77} symbol="fn slice" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={83} symbol="fn index_set" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-index-takes-a-number-per-dimension.kav`} label="tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-slice-keeps-its-dimensions.kav`} label="slices, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-index-set-changes-one-number.kav`} label="setting, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-get-gives-none-out-of-range.kav`} label="get, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-len-dims-and-item.kav`} label="len, dims, item, tested" />
      </div>
    </>
  ),
};
