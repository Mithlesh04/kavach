import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- Matrices ----------
//
// Spec 2.17 (decision 0136): a matrix is a Tensor<T, dims: 2> from
// std.tensor, Kavach's one array type. `Matrix<T>` and `std.matrix` are
// gone, and naming them is a compile error that says what to write.

const SPEC = "docs/spec/02-types/17-matrices.md";
const TESTS = "tests/conformance/02-types";

export const matricesDoc: LessonDoc = {
  lead: "A matrix is a tensor of two dimensions, Tensor<T, dims: 2>, from std.tensor. a @ b is the matrix product, and + - * work item by item.",
  simple: <>A matrix is a table of numbers: rows and columns, like a spreadsheet. Many calculations are a matrix times another: scores times weights, points times a turn, inputs times a layer of a neural network. <C>a @ b</C> does that in one step.</>,
  minutes: 4,
  before: ["types/operators-for-your-types", "collections/lists"],
  body: (
    <>
      <Ex name="matrices/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>tensor.of_rows</C> takes the rows. They must all be as long.</li>
        <li><C>a @ b</C> is the matrix product: each item is a row of <C>a</C> times a column of <C>b</C>, added up.</li>
        <li><C>a * b</C> multiplies item by item, as do <C>+</C>, <C>-</C> and <C>-a</C>.</li>
        <li><C>a[i, j]</C> reads one item, counting from 0, and <C>shape</C> says how many rows and columns there are.</li>
      </ul>
      <P>
        A matrix is an ordinary <Link href="/docs/standard-library/tensors/">tensor</Link>, so everything a tensor has
        works on it: slices, masks, sums along an axis, sorting, and the{" "}
        <Link href="/docs/standard-library/tensors/linear-algebra/">linear algebra</Link> of inverses, solving and
        decompositions.
      </P>
      <H2>A use: weighted scores</H2>
      <P>
        Each row is a student and each column a test. Times a column of weights, the product gives each student&apos;s
        final score in one step. <C>sum(axis: 0)</C> adds each column, the total of each test:
      </P>
      <Ex name="matrices/grades" />
      <H2>Sizes must fit</H2>
      <P>
        <C>a @ b</C> needs as many columns in <C>a</C> as rows in <C>b</C>. A size that doesn&apos;t fit stops the
        program with both shapes, at the line of your code that asked for the product:
      </P>
      <Ex name="matrices/sizes.panics" />
      <P><C>@</C> is only the matrix product. Two numbers are multiplied with <C>*</C>, and the compiler says so:</P>
      <Ex name="matrices/numbers.fails" />
      <H2>The same bits everywhere</H2>
      <P>
        Each item of <C>a @ b</C> adds its products in one fixed order, so a <C>Float</C> product gives exactly the same
        bits on every machine and in every build, as a loop written by hand would. <C>a.matmul(b, fast: true)</C> is
        free to add in another order, for the full speed of the machine; a <C>Float</C> result may then differ in its
        last bits. Whole numbers give the same result either way:
      </P>
      <Ex name="matrices/fast" />
      <P>
        The product spreads its rows over the cores the program may use: all of them, or as many as{" "}
        <C>[run] cores</C> and <C>on cores(n)</C> allow. Its speed against other languages is being measured again,
        with each side on the same number of cores.
      </P>
      <H2>No Matrix type</H2>
      <P>
        <C>Tensor</C> is Kavach&apos;s one array type, so there is no separate <C>Matrix</C>. Naming the old type or its
        module is a compile error that says what to write instead:
      </P>
      <Ex name="matrices/gone.fails" />
      <Ex name="matrices/gone-type.fails" />
      <H2>@ for your own types</H2>
      <P>
        <C>@</C> comes from the interface <C>MatMul</C>, like the{" "}
        <Link href="/docs/types/operators-for-your-types/">other operators</Link>. It goes before <C>+</C> and{" "}
        <C>-</C>, as <C>*</C> does, and <C>x @= y</C> is <C>x = x @ y</C>:
      </P>
      <Ex name="types/operators-matmul" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.17 · matrices" />
        <SrcRef file="docs/decisions/0136-tensor-the-one-array-type.md" label="decision 0136" />
        <SrcRef file={`${TESTS}/the-matrix-type-is-gone.kav`} label="Matrix gone, tested" />
        <SrcRef file={`${TESTS}/std-matrix-is-gone.kav`} label="std.matrix gone, tested" />
        <SrcRef file={`${TESTS}/matmul-is-not-for-numbers.kav`} label="@ on numbers, tested" />
      </div>
    </>
  ),
  tryIt: {
    task: <>Turn the points (1, 0), (0, 2) and (3, 1) a quarter turn to the left: put them in a tensor as columns, and multiply <C>tensor.of_rows([[0, -1], [1, 0]])</C> by it. Print the result&apos;s rows.</>,
    answer: <P><C>tensor.of_rows([[0, -1], [1, 0]]) @ tensor.of_rows([[1, 0, 3], [0, 2, 1]])</C> gives <C>[[0, -2, -1], [1, 0, 3]]</C>: the points (0, 1), (-2, 0) and (-1, 3).</P>,
  },
};
