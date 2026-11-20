import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Tensors: the overview ----------
//
// Spec 14.33 (decision 0136): Tensor<T>, Kavach's one array type. The
// overview has the matrix product and NumPy's names; making, indexing,
// arithmetic, masks, reductions and sorting have pages of their own
// (stdlib-tensor-*.tsx). Every table comes from the spec itself, only the
// rows that are built (lib/built-spec.ts).

export const TENSOR_SPEC = "14-standard-library/33-tensor.md";
export const TENSOR_TESTS = "tests/conformance/14-standard-library";
export const TENSOR_SRC = "stdlib/tensor";

/** The spec's table under a `##` heading of 14.33, without what a maintainer chose not to state yet. */
export function tensorTable(heading: string) {
  const t = specTables(TENSOR_SPEC, "14.33").find((x) => x.heading === heading);
  if (!t) throw new Error(`spec 14.33: no table under "${heading}"`);
  // random_int's cap of 65,536 numbers is going away (a maintainer's choice, 2026-11-19): not stated.
  const rows = t.rows.map((r) => r.map((c) => c.replace(/; at most 65,536 numbers at a time, as `random\.bytes\(n\)` \(14\.15\)/, "")));
  return <Table header={t.header} rows={rows} wrapFirst />;
}

/** The sub-pages, in order, for the overview's list. */
const PAGES: [string, string, string][] = [
  ["making-tensors", "Making tensors", "of, of_rows, zeros, eye, arange, linspace, random numbers, converting"],
  ["indexing", "Indexing", "t[i, j], #k, slices of each dimension, setting a number, get and item"],
  ["arithmetic-and-broadcasting", "Arithmetic and broadcasting", "+ - * / % ** item by item, a number for every item, NumPy's broadcasting"],
  ["masks", "Masks", "comparisons that give a Tensor<Bool>, t[mask], tensor.where, eq"],
  ["reductions", "Reductions", "sum, min, max, mean, std along an axis or over every number; exact float sums"],
  ["sorting-and-joining", "Sorting and joining", "sort, argsort, unique, percentile, concatenate, stack, split"],
];

export const tensorDoc: LessonDoc = {
  lead: "std.tensor gives Tensor<T>: numbers in any number of dimensions, Kavach's one array type, as NumPy's ndarray is Python's. Every operation checks its shapes, and every result is the same on every machine.",
  simple: <>A list is numbers in a line, and a grid is numbers in rows and columns. A tensor is the same idea with as many dimensions as you need: a batch of images is four (images, height, width, colour).</>,
  minutes: 6,
  before: ["types/generics", "collections/indexing-and-slicing"],
  body: (
    <>
      <P>
        <C>use std.tensor</C> brings in the module, written in Kavach in <C>stdlib/tensor/</C>. A tensor has a{" "}
        <C>shape</C>, its size in each dimension, and its <C>items</C>, every number in one list, the last
        dimension&apos;s next to each other (row by row for a matrix):
      </P>
      <Ex name="tensor/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>T</C> is a number type (<C>Float</C>, <C>Float32</C>, a whole-number type or <C>Decimal</C>) or <C>Bool</C>. A tensor of <C>Bool</C> is a <Link href="/docs/standard-library/tensors/masks/">mask</Link>.</li>
        <li><C>dims()</C> is how many dimensions it has, and <C>len()</C> how many numbers it holds.</li>
        <li><C>x[1, 2]</C> takes one index per dimension, counted from 0: row 1, column 2.</li>
        <li><C>reshape</C> keeps the numbers in order and changes the shape; <C>transpose</C> swaps the last two dimensions.</li>
      </ul>
      <P>A tensor of anything else is a compile error:</P>
      <Ex name="tensor/numbers-only.fails" />
      <H2>The pages on tensors</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {PAGES.map(([slug, title, what]) => (
          <li key={slug}>
            <Link href={`/docs/standard-library/tensors/${slug}/`}>{title}</Link>: {what}
          </li>
        ))}
      </ul>
      <H2>The matrix product</H2>
      <P>
        <C>a @ b</C> is the matrix product, as NumPy&apos;s <C>matmul</C>. Two matrices, <C>[n, k]</C> and{" "}
        <C>[k, m]</C>, give <C>[n, m]</C>; <C>@=</C> is <C>a = a @ b</C>:
      </P>
      <Ex name="tensor/matmul" />
      <P>The inner sizes must match. Another pair is a panic that names both shapes:</P>
      <Ex name="tensor/matmul.panics" />
      <P>
        With more than two dimensions, the last two are the matrices and the others are a batch: <C>[2, 2, 2] @ [2, 2, 2]</C>{" "}
        is one product for each of the 2. Both batches have the same shape:
      </P>
      <Ex name="tensor/batches" />
      <P>
        Each number of <C>a @ b</C> adds its products in one order, a multiply then an add, never fused. The work is
        still spread over the cores, so the result has the same bits on every machine and at every thread count.
        A layer of a small neural network is a product, a sum and a function on every number, here ReLU:
      </P>
      <Ex name="tensor/layer" />
      <H2>A tensor is a value</H2>
      <P>
        Like a <Link href="/docs/memory/values-not-references/">list</Link>, a tensor is a value: a copy shares its
        numbers until one of them changes, so changing <C>b</C> leaves <C>a</C> as it was:
      </P>
      <Ex name="tensor/copy" />
      <H2>NumPy&apos;s names</H2>
      <P>Each job has one name. Where NumPy&apos;s name differs, it is a compile error that names Kavach&apos;s:</P>
      {tensorTable("NumPy's names")}
      <Ex name="tensor/numpy-name.fails" />
      <Note>
        The spec&apos;s tables write how many dimensions a result has in its type, as{" "}
        <C>Tensor&lt;T, dims: 2&gt;</C>. Writing <C>dims:</C> or <C>shape:</C> in a type is not built yet: write{" "}
        <C>Tensor&lt;T&gt;</C>, and the shapes are checked while the program runs. Not built yet either: <C>@</C>{" "}
        of a vector, or of batches of different shapes, linear algebra (<C>inv</C>, <C>solve</C>,{" "}
        <C>det</C>, <C>eig</C>, <C>svd</C>), the FFT and the GPU.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · std.tensor" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={21} symbol="type Tensor" />
        <SrcRef file={`${TENSOR_SRC}/ops.kav`} line={118} symbol="fn matmul" />
        <SrcRef file="docs/decisions/0136-tensor-the-one-array-type.md" label="decision 0136" />
        <SrcRef file="docs/decisions/0125-kavach-for-ai-work.md" label="decision 0125" />
        <SrcRef file={`${TENSOR_TESTS}/tensors-check-their-shapes.kav`} label="shapes, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-matmul-of-equal-batches.kav`} label="batches, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-of-text-is-an-error.kav`} label="numbers only, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-numpy-names-are-errors.kav`} label="NumPy's names, tested" />
      </div>
    </>
  ),
};
