import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { fileFigure, fileTablesAt } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Matrices ----------
//
// Spec 2.17: Matrix<T> in std.matrix, with `a @ b` for the product and
// + - * working item by item. The table of what a matrix has is the
// spec's own, read at build time.

const SPEC = "docs/spec/02-types/17-matrices.md";
/**
 * Spec 2.17 now says Tensor replaces Matrix (decision 0136), which isn't built
 * yet, so this page reads 2.17 as it was before. When Tensor's code lands,
 * this page goes and the tensor page takes its place.
 */
const PINNED = "81e236e3";
const TESTS = "tests/conformance/02-types";
const BENCH = "benchmarks/math/README.md";

/** The matmul times of the math benchmarks, read from its README. */
const matmul = () => ({
  kavach: fileFigure(BENCH, /`matmul`, measured [^:]*: Kavach (\d+ ms)/),
  cpp: fileFigure(BENCH, /`matmul`, measured [^:]*: Kavach \d+ ms, C\+\+ (\d+ ms)/),
});

/** The spec's table of ways to make and use a matrix. */
const written = () => {
  const t = fileTablesAt(PINNED, SPEC).find((x) => x.header[0] === "Written");
  if (!t) throw new Error(`${SPEC} at ${PINNED}: no Written table`);
  return t;
};

export const matricesDoc: LessonDoc = {
  lead: "A Matrix<T> is a grid of numbers in std.matrix. a @ b is the matrix product, + - * work item by item, and a product gives the same bits on every machine.",
  simple: <>A matrix is a table of numbers: rows and columns, like a spreadsheet. Many calculations are a matrix times another: scores times weights, points times a turn, inputs times a layer of a neural network. <C>a @ b</C> does that in one step, spread over every core.</>,
  minutes: 6,
  before: ["types/operators-for-your-types", "collections/lists"],
  body: (
    <>
      <Ex name="matrices/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>matrix.of</C> takes the rows. They must all be as long, or the program panics, saying both lengths.</li>
        <li><C>a @ b</C> is the matrix product: each item is a row of <C>a</C> times a column of <C>b</C>, added up.</li>
        <li><C>a * b</C> multiplies item by item, as do <C>+</C>, <C>-</C> and <C>-a</C>.</li>
        <li><C>items</C> holds every item row by row, in one flat list. <C>at(i, j)</C> reads one, counting from 0.</li>
      </ul>
      <H2>A use: weighted scores</H2>
      <P>
        Each row is a student and each column a test. Times a column of weights, the product gives each student&apos;s
        final score in one step:
      </P>
      <Ex name="matrices/grades" />
      <H2>Making, reading and changing one</H2>
      <Ex name="matrices/make" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>slice(0, 2, 1, 3)</C> takes rows 0 up to 2 and columns 1 up to 3, not including the ends, as a new matrix.</li>
        <li><C>matrix.identity(n, one)</C> is given its <C>one</C>, so it works for every number type: <C>1</C>, <C>1.0</C> or a <C>Float32</C>.</li>
        <li>
          <C>m.index(i, j)</C> says where an item is in <C>items</C>, so <C>m.items[m.index(i, j)] = v</C> changes it. The
          matrix must be a <C>let</C>; nothing is copied when nothing else holds it.
        </li>
      </ul>
      <H2>Everything a matrix has</H2>
      <Table header={written().header} rows={written().rows} />
      <H2>Sizes must fit</H2>
      <P>
        <C>a @ b</C> needs as many columns in <C>a</C> as rows in <C>b</C>. A size that doesn&apos;t fit breaks a{" "}
        <Link href="/docs/errors/contracts/require/">require</Link> and stops the program, with the rule it broke:
      </P>
      <Ex name="matrices/sizes.panics" />
      <P>
        <C>@</C> is only the matrix product. Two numbers are multiplied with <C>*</C>, and the compiler says so:
      </P>
      <Ex name="matrices/numbers.fails" />
      <P>
        The type is <C>{"Matrix<T: Number>"}</C>: its items are numbers, so a matrix of anything else is a compile
        error:
      </P>
      <Ex name="matrices/text.fails" />
      <SrcRef file="tests/conformance/02-types/matrix-holds-only-numbers.kav" label="numbers only, tested" />
      <H2>The same bits everywhere</H2>
      <P>
        Each item of <C>a @ b</C> adds its products in one fixed order, a multiply and then an add each time. So a{" "}
        <C>Float</C> product gives exactly the same bits on every machine and in every build, as a loop written by hand
        would. The work is still spread over the cores and the processor&apos;s vector units.
      </P>
      <P>
        <C>a.product(b, fast: true)</C> is free to add in another order, for the full speed of the machine. A{" "}
        <C>Float</C> result may then differ in its last bits between machines. Whole numbers give the same result
        either way:
      </P>
      <Ex name="matrices/fast" />
      <P>
        The exact product is fast too. Multiplying two 1024 × 1024 <C>Float</C> matrices 4 times takes{" "}
        {matmul().kavach} in Kavach, against {matmul().cpp} for the plain C++ loop on one core, with the same bits.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={BENCH} label="math benchmarks" />
        <SrcRef file="benchmarks/math/kavach/matmul.kav" label="matmul.kav" />
        <SrcRef file="benchmarks/math/cpp/matmul.cpp" label="matmul.cpp" />
      </div>
      <H2>@ for your own types</H2>
      <P>
        <C>@</C> comes from the interface <C>MatMul</C>, like the{" "}
        <Link href="/docs/types/operators-for-your-types/">other operators</Link>. It goes before <C>+</C> and{" "}
        <C>-</C>, as <C>*</C> does, and <C>x @= y</C> is <C>x = x @ y</C>:
      </P>
      <Ex name="types/operators-matmul" />
      <Note>
        Not yet: products on the GPU, views that share items with the matrix they came from instead of copies, and more
        than two dimensions.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.17 · matrices" />
        <SrcRef file="docs/decisions/0111-matrices.md" label="decision 0111" />
        <SrcRef file={`${TESTS}/matrices-multiply-and-work-item-by-item.kav`} label="tested" />
        <SrcRef file={`${TESTS}/matrix-products-give-the-same-bits.kav`} label="same bits, tested" />
        <SrcRef file={`${TESTS}/matrix-sizes-must-match.kav`} label="sizes, tested" />
        <SrcRef file={`${TESTS}/matmul-calls-the-types-own-method.kav`} label="MatMul, tested" />
        <SrcRef file="stdlib/matrix/matrix.kav" label="matrix.kav" />
        <SrcRef file="runtime/matrix/src/product.rs" label="product.rs" />
      </div>
    </>
  ),
  tryIt: {
    task: <>Turn the points (1, 0), (0, 2) and (3, 1) a quarter turn to the left: put them in a matrix as columns, and multiply <C>matrix.of([[0, -1], [1, 0]])</C> by it. Print the result&apos;s rows.</>,
    answer: <P><C>matrix.of([[0, -1], [1, 0]]) @ matrix.of([[1, 0, 3], [0, 2, 1]])</C> gives <C>[[0, -2, -1], [1, 0, 3]]</C>: the points (0, 1), (-2, 0) and (-1, 3).</P>,
  },
};
