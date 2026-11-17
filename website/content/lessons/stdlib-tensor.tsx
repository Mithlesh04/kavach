import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Tensors ----------
//
// Spec 14.33. The calls table comes from the spec itself.

const FILE = "14-standard-library/33-tensor.md";

function CallsTable() {
  const t = specTables(FILE, "14.33")[0];
  if (!t || !t.rows[0][0].includes("tensor.of")) throw new Error("spec 14.33: the first table no longer starts with tensor.of");
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const tensorDoc: LessonDoc = {
  lead: "std.tensor gives Tensor<T>: numbers in any number of dimensions, for machine learning and science. Every operation checks its shapes, and every result is the same on every machine.",
  simple: <>A list is numbers in a line; a <Link href="/docs/types/matrices/">matrix</Link> is numbers in a grid. A tensor is the same idea with as many dimensions as you need: a batch of images is four (images, height, width, colour).</>,
  minutes: 5,
  before: ["types/matrices", "types/generics"],
  body: (
    <>
      <P>
        <C>use std.tensor</C> brings in the module, written in Kavach in <C>stdlib/tensor/tensor.kav</C>. A
        tensor has a <C>shape</C>, its size in each dimension, and its <C>items</C>, every number in one list,
        the last dimension&apos;s next to each other:
      </P>
      <Ex name="tensor/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>tensor.of_rows</C> makes two dimensions from rows; every row must be as long.</li>
        <li><C>get</C> takes one index per dimension: <C>[1, 2]</C> is row 1, column 2, counting from 0.</li>
        <li><C>reshape</C> keeps the numbers in order and changes the shape, and <C>transpose</C> swaps the last two dimensions.</li>
      </ul>
      <H2>Making tensors</H2>
      <P>
        <C>T</C> is any number type: <C>Float</C>, <C>Float32</C>, the whole-number types or <C>Decimal</C>.{" "}
        <C>zeros</C> and <C>ones</C> name it, as nothing else says which; <C>full</C>, <C>of</C> and{" "}
        <C>of_rows</C> take it from their numbers:
      </P>
      <Ex name="tensor/make" />
      <P>Anything but a number is a compile error:</P>
      <Ex name="tensor/numbers-only.fails" />
      <CallsTable />
      <H2>A layer of a neural network</H2>
      <P>
        <C>@</C> is the matrix product, as for <Link href="/docs/types/matrices/">matrices</Link>, and{" "}
        <C>+</C> adds item by item. <C>map</C> runs a function on every number, here ReLU, which keeps the
        positive ones:
      </P>
      <Ex name="tensor/layer" />
      <P>
        Two rows of four inputs times four-by-three weights gives two rows of three outputs. Each output is
        added up in order, so the result is the same on every machine and at every thread count.
      </P>
      <H2>More dimensions</H2>
      <P>
        With more than two dimensions, <C>@</C> multiplies the last two, once for each of the others, which
        must match. Three 2 × 2 matrices times three more give three products:
      </P>
      <Ex name="tensor/batches" />
      <H2>When shapes don&apos;t fit</H2>
      <P>A shape that doesn&apos;t fit is a panic that names both shapes:</P>
      <Ex name="tensor/shape.panics" />
      <P>So is a reshape to a different count of numbers, or rows of different lengths:</P>
      <Ex name="tensor/reshape.panics" />
      <Ex name="tensor/rows.panics" />
      <H2>A tensor is a value</H2>
      <P>
        Like a <Link href="/docs/memory/values-not-references/">list</Link>, a tensor is a value: a copy shares its
        numbers until one of them changes, so changing <C>b</C> leaves <C>a</C> as it was:
      </P>
      <Ex name="tensor/copy" />
      <P>
        Not yet: adding a number or a smaller tensor to every row (broadcasting), sums along one dimension,{" "}
        <C>mean</C>, shapes in the type, <C>Float16</C>, and running on the GPU. The spec lists them as open
        questions.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.33 · std.tensor" />
        <SrcRef file="stdlib/tensor/tensor.kav" label="tensor.kav" />
        <SrcRef file="tests/conformance/14-standard-library/tensors-check-their-shapes.kav" label="shapes, tested" />
        <SrcRef file="docs/decisions/0125-kavach-for-ai-work.md" label="decision 0125" />
      </div>
    </>
  ),
};
