import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { TENSOR_SPEC, TENSOR_SRC, TENSOR_TESTS, tensorTable } from "./stdlib-tensor";

// ---------- Tensors: arithmetic, broadcasting and masks ----------
//
// Spec 14.33's "Item by item, and broadcasting" and "Comparisons, masks and
// `where`".

export const tensorMathDoc: LessonDoc = {
  lead: "+ - * / % and ** work item by item on two tensors, or on a tensor and a number. Tensors of different shapes are broadcast to one, as NumPy does.",
  simple: <>Adding two grids of the same size adds each pair of numbers. Adding one row to a whole grid adds it to every row: the row is used again for each, without being copied.</>,
  minutes: 6,
  before: ["standard-library/tensors"],
  body: (
    <>
      <Ex name="tensor/arithmetic" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each operator takes a tensor or a number on the right; a number is used for every item.</li>
        <li>Each item works as its numbers would alone: for whole numbers, <C>/</C> cuts off the fraction, and division by zero and overflow panic.</li>
        <li><C>t += x</C> and the other <C>op=</C> forms are <C>t = t op x</C>, so <C>t</C> must be a <C>let</C>.</li>
      </ul>
      <P>The number goes on the right, as for every operator of a type of its own:</P>
      <Ex name="tensor/left.fails" />
      <P>
        Both sides have one item type: a <C>Tensor&lt;Int&gt;</C> and a <C>Float</C> don&apos;t mix, as{" "}
        <C>Int</C> and <C>Float</C> don&apos;t. Convert first, with <C>Tensor&lt;Float&gt;(counts)</C>:
      </P>
      <Ex name="tensor/mix.fails" />
      <H2>Broadcasting</H2>
      <P>Two tensors of different shapes work item by item when their shapes fit:</P>
      <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
        <li>Line the two shapes up from the right.</li>
        <li>The one with fewer dimensions gets sizes of 1 on its left.</li>
        <li>Two sizes fit when they are equal, or when one of them is 1. The result takes the larger.</li>
        <li>A size of 1 is used again for every place of the other size. Nothing is copied to do so.</li>
      </ol>
      <Ex name="tensor/broadcast" />
      <P>Shapes that don&apos;t fit are a panic naming both:</P>
      <Ex name="tensor/shape.panics" />
      <H2>Every operator</H2>
      {tensorTable("Item by item, and broadcasting")}
      <P>
        <C>Tensor&lt;T&gt;</C> implements each operator twice, once for a tensor on the right and once for a number.
        A program&apos;s own types can do the same: see{" "}
        <Link href="/docs/types/operators-for-your-types/">Operators for Your Types</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · item by item" />
        <SrcRef file={`${TENSOR_SRC}/ops.kav`} line={14} symbol="fn add" />
        <SrcRef file={`${TENSOR_SRC}/ops.kav`} line={109} symbol="fn clamp" />
        <SrcRef file={`${TENSOR_SRC}/shapes.kav`} line={60} symbol="fn broadcast" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-broadcasting-follows-numpy.kav`} label="broadcasting, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-number-on-the-right.kav`} label="a number, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-shapes-that-dont-broadcast-panic.kav`} label="shapes that don't fit, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-whole-numbers-and-floats-dont-mix.kav`} label="one item type, tested" />
      </div>
    </>
  ),
};

export const tensorMasksDoc: LessonDoc = {
  lead: "t > 0.5 compares item by item and gives a mask, a Tensor<Bool>. t[mask] picks the numbers where it is true, and tensor.where chooses between two tensors.",
  simple: <>A mask is a grid of yes and no, one for each number. It can pick the numbers you want out of a tensor, or replace them, without a loop.</>,
  minutes: 4,
  before: ["standard-library/tensors/arithmetic-and-broadcasting"],
  body: (
    <>
      <Ex name="tensor/masks" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>&lt; &lt;= &gt; &gt;=</C> take a tensor or a number on the right, and give a mask of the broadcast shape.</li>
        <li><C>a == b</C> compares whole tensors, giving one <C>Bool</C>, as for every type. Equal item by item is <C>a.eq(b)</C>, and <C>a.ne(b)</C> is not equal.</li>
        <li>A <C>NaN</C> equals nothing, so <C>t.eq(t)</C> is <C>false</C> where <C>t</C> is <C>NaN</C>.</li>
        <li><C>t[mask] = x</C> sets those numbers of a <C>let</C> tensor.</li>
        <li><C>any</C>, <C>all</C> and <C>count</C> take a test, as a list&apos;s do.</li>
      </ul>
      <P>A mask has the shape of the tensor it picks from; another shape is a panic naming both:</P>
      <Ex name="tensor/mask-shape.panics" />
      <H2>Every call</H2>
      {tensorTable("Comparisons, masks and `where`")}
      <P>
        The comparisons come through the <C>Compare</C> interface, which gives each of <C>&lt; &lt;= &gt; &gt;=</C>{" "}
        its own method and result: see <Link href="/docs/types/operators-for-your-types/">Operators for Your Types</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · masks" />
        <SrcRef file={`${TENSOR_SRC}/ops.kav`} line={99} symbol="fn eq" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={65} symbol="fn index (a mask)" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={344} symbol="fn where" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-comparison-gives-a-mask.kav`} label="comparisons, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-mask-picks-numbers.kav`} label="masks, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-where-chooses-item-by-item.kav`} label="where, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-eq-compares-item-by-item.kav`} label="eq, tested" />
      </div>
    </>
  ),
};
