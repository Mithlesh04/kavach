import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { TENSOR_SPEC, TENSOR_SRC, TENSOR_TESTS, tensorTable } from "./stdlib-tensor";

// ---------- Tensors: reductions, sorting and joining ----------
//
// Spec 14.33's "Reductions by axis" and "Sorting, joining and splitting".

export const tensorReduceDoc: LessonDoc = {
  lead: "sum, prod, min, max, mean, var and std work on every number, or along one axis: m.sum(axis: 0) sums each column. A float sum is exact, so it is the same on every machine.",
  simple: <>Adding up a whole grid gives one number. Adding up each column gives a row of totals, one for each column: that is a sum along an axis.</>,
  minutes: 5,
  before: ["standard-library/tensors"],
  body: (
    <>
      <Ex name="tensor/reduce" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Without <C>axis:</C>, a reduction gives one number. With it, a tensor with that dimension taken out: <C>axis: 0</C> works down each column of a matrix, <C>axis: 1</C> along each row.</li>
        <li><C>min</C>, <C>max</C>, <C>mean</C>, <C>var</C> and <C>std</C> of every number give an optional, <C>none</C> when there are no numbers. <C>argmin</C> and <C>argmax</C> give the place of the first smallest or largest.</li>
        <li><C>var</C> and <C>std</C> divide by the count minus 1, as for lists; <C>correction: 0</C> is NumPy&apos;s form.</li>
        <li><C>cumsum</C> gives the running sums along the last axis, or along <C>axis:</C>, the shape kept.</li>
        <li><C>NaN</C> comes after every number, as in a list&apos;s <C>min</C> and <C>max</C>.</li>
      </ul>
      <H2>Exact float sums</H2>
      <P>
        A float sum is the true sum of its numbers, rounded once. No order changes it, so it has the same bits on
        every machine, core count and device. <C>mean</C> is that sum divided by the count, rounded once:
      </P>
      <Ex name="tensor/exact-sum" />
      <P>
        Adding one at a time would lose the <C>1.0</C> next to <C>1e20</C>, and give <C>0.0</C> or <C>1.0</C>{" "}
        depending on the order.
      </P>
      <H2>The norm</H2>
      <P>
        <C>norm()</C> is the square root of the sum of the squares, the length of a vector. With <C>axis:</C>, it is
        one for each row or column. The squares are summed exactly, as above, then the root is taken:
      </P>
      <Ex name="tensor/norm" />
      <P>
        It needs <C>T: Floating</C>, and is listed with{" "}
        <Link href="/docs/standard-library/tensors/linear-algebra/">linear algebra</Link>, as in the spec.
      </P>
      <H2>What is refused</H2>
      <P><C>mean</C>, <C>var</C>, <C>std</C> and <C>percentile</C> need a <C>T: Floating</C>. Convert whole numbers first, with <C>Tensor&lt;Float&gt;(t)</C>:</P>
      <Ex name="tensor/mean-int.fails" />
      <P>Axes count from 0, so a written negative one is a compile error, as <C>xs[-1]</C> is:</P>
      <Ex name="tensor/negative-axis.fails" />
      <P>Along an axis of size 0, <C>sum</C> gives 0 and <C>prod</C> 1; <C>min</C>, <C>max</C>, <C>argmin</C>, <C>argmax</C> and <C>mean</C> are a panic naming the shape:</P>
      <Ex name="tensor/empty-axis.panics" />
      <H2>Every reduction</H2>
      {tensorTable("Reductions by axis")}
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · reductions" />
        <SrcRef file={`${TENSOR_SRC}/reduce.kav`} line={17} symbol="fn sum" />
        <SrcRef file={`${TENSOR_SRC}/reduce.kav`} line={73} symbol="fn mean" />
        <SrcRef file={`${TENSOR_SRC}/exact.kav`} line={202} symbol="fn sum (exact)" />
        <SrcRef file={`${TENSOR_SRC}/reduce.kav`} line={104} symbol="fn norm" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-sum-along-an-axis.kav`} label="axes, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-float-sum-is-exact.kav`} label="exact sums, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-mean-var-and-std.kav`} label="mean, var, std, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-min-max-and-their-places.kav`} label="min and max, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-cumsum-runs-along-the-last-axis.kav`} label="cumsum, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-norm-is-the-root-of-the-squares.kav`} label="norm, tested" />
      </div>
    </>
  ),
};

export const tensorSortDoc: LessonDoc = {
  lead: "sort, argsort, unique and percentile order a tensor's numbers; concatenate, stack, split and split_at join tensors and cut them apart.",
  simple: <>Sorting puts numbers smallest first. Joining puts tensors end to end, or stacks them like sheets of paper; splitting cuts one into equal pieces.</>,
  minutes: 4,
  before: ["standard-library/tensors"],
  body: (
    <>
      <Ex name="tensor/sort" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>sort</C> and <C>argsort</C> work along the last axis, or along <C>axis:</C>. The sort is stable, so <C>argsort</C> gives the same indexes everywhere; <C>NaN</C> goes last.</li>
        <li><C>unique</C> gives every number once, sorted, all <C>NaN</C>s as one.</li>
        <li><C>percentile(q)</C> interpolates between the two nearest numbers, as NumPy&apos;s default; <C>q</C> is from 0 to 100.</li>
      </ul>
      <H2>Joining and splitting</H2>
      <Ex name="tensor/join" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>concatenate</C> puts the parts one after another along an axis; their other sizes must match.</li>
        <li><C>stack</C> puts parts of one shape along a new axis.</li>
        <li><C>split(count)</C> cuts equal parts, and <C>split_at(points)</C> cuts before each point.</li>
      </ul>
      <P>A size that the count doesn&apos;t divide is a panic naming both:</P>
      <Ex name="tensor/split.panics" />
      <H2>Every call</H2>
      {tensorTable("Sorting, joining and splitting")}
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · sorting and joining" />
        <SrcRef file={`${TENSOR_SRC}/order.kav`} line={15} symbol="fn sort" />
        <SrcRef file={`${TENSOR_SRC}/order.kav`} line={47} symbol="fn percentile" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={358} symbol="fn concatenate" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-sort-and-argsort.kav`} label="sorting, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-percentile-interpolates.kav`} label="percentile, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-concatenate-and-stack.kav`} label="joining, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-split-cuts-equal-parts.kav`} label="splitting, tested" />
      </div>
    </>
  ),
};
