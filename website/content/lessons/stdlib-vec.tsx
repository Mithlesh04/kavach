import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Vectors ----------
//
// Spec 14.28. Its four tables have no headings of their own, so each is
// picked by its place: vectors, then Mat4, Mat3 and Quat.

const FILE = "14-standard-library/28-vec.md";

function VecTable({ n, first }: { n: number; first: string }) {
  const t = specTables(FILE, "14.28")[n];
  if (!t || !t.rows[0][0].includes(first)) throw new Error(`spec 14.28: table ${n} no longer starts with ${first}`);
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const vecDoc: LessonDoc = {
  lead: "std.vec has the math of games, graphics and simulations: Vec2, Vec3 and Vec4, the matrices Mat3 and Mat4, and the rotation Quat. Vectors work over any number type; matrices over Float or Float32.",
  simple: <>A vector is an arrow: which way, and how far. A game keeps every position and speed as one. A matrix moves, turns and scales many arrows the same way at once, as a camera does to a whole scene.</>,
  minutes: 6,
  before: ["types/generics", "basics/math"],
  body: (
    <>
      <Ex name="vec/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Vec2</C>, <C>Vec3</C> and <C>Vec4</C> are records of <C>x</C>, <C>y</C>, <C>z</C> and <C>w</C>, as many as their number says.</li>
        <li><C>+</C>, <C>-</C> and <C>*</C> or <C>/</C> by a number work on each part.</li>
        <li><C>normalized()</C> keeps the direction and makes the length 1: the way to go, without how far.</li>
        <li><C>cross</C>, for <C>Vec3</C>, gives a vector at right angles to both.</li>
      </ul>
      <VecTable n={0} first="a + b" />
      <H2>Moving, turning and cameras</H2>
      <P>
        A <C>Mat4</C> is four columns, each a <C>Vec4</C>, as GPUs lay it out. <C>m @ v</C> transforms a point, and{" "}
        <C>a.product(b)</C> joins two matrices: <C>b</C> first, then <C>a</C>. A <C>Quat</C> is a rotation:
      </P>
      <Ex name="vec/transform" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A point has <C>w: 1.0</C>, so translation moves it.</li>
        <li>Angles are in radians, as <C>std.math</C>&apos;s are: a quarter turn is <C>math.pi / 2.0</C>.</li>
        <li><C>Mat4.perspective</C> gives depth from 0 to 1, as GPUs take it; see <Link href="/docs/speed/gpu/drawing-triangles/">Drawing triangles</Link>.</li>
      </ul>
      <VecTable n={1} first="Mat4.identity()" />
      <VecTable n={3} first="Quat.identity()" />
      <H2>3 × 3 matrices</H2>
      <P>
        A <C>Mat3</C> is three columns, <C>c0</C> to <C>c2</C>, each a <C>Vec3</C>. It turns and scales without
        moving: for normals in lighting, or for a 2D transform. A <C>Mat4</C>&apos;s <C>mat3()</C> keeps its turning
        and scaling and drops its moving:
      </P>
      <Ex name="vec/mat3" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>determinant()</C> is how much it scales volume: 2 × 3 × 4 = 24 here. It is 0 when the matrix flattens space.</li>
        <li><C>inverse()</C> undoes the matrix, or gives <C>none</C> when the determinant is 0, as nothing can undo a flattening.</li>
        <li>For lighting, the normals&apos; matrix is <C>model.mat3().inverse()</C>, transposed.</li>
      </ul>
      <VecTable n={2} first="Mat3.identity()" />
      <H2>Whole-number vectors</H2>
      <P>
        A vector&apos;s numbers may be of any number type, so <C>Vec2&lt;Int&gt;</C> is a cell of a grid. A literal
        gives the type: <C>Vec2(x: 3, y: 4)</C> is a <C>Vec2&lt;Int&gt;</C>. The operators, <C>dot</C>,{" "}
        <C>cross</C> and <C>length_squared</C> work on whole numbers:
      </P>
      <Ex name="vec/whole" />
      <P>
        <C>length</C>, <C>normalized</C>, <C>distance</C> and <C>lerp</C> give fractions, so they need a{" "}
        <C>Float</C> or <C>Float32</C>. They are methods with a{" "}
        <Link href="/docs/types/generics/">where bound</Link>, and the error names it:
      </P>
      <Ex name="vec/whole-length.fails" />
      <H2>Float or Float32</H2>
      <P>
        The program picks the number type: <C>Vec3&lt;Float32&gt;</C>, as GPUs and game engines use, or{" "}
        <C>Vec3&lt;Float&gt;</C>. Each is compiled for its type, so it runs as code written for that type would:
      </P>
      <Ex name="vec/float32" />
      <Note>
        <C>std.vec</C> is written in Kavach, over the number bounds of{" "}
        <Link href="/docs/types/generics/">Generics</Link>. A matrix&apos;s or quaternion&apos;s type is{" "}
        <C>Float</C> or <C>Float32</C>, and a literal gives it: <C>Vec2(x: 1.0, y: 2.0)</C> is a{" "}
        <C>Vec2&lt;Float&gt;</C>.
      </Note>
      <H2>On the GPU</H2>
      <P>
        In a <C>gpu fn</C>, <C>Vec2</C> to <C>Vec4</C>, <C>Mat3</C> and <C>Mat4</C> of <C>Float32</C> are the GPU&apos;s own
        vector and matrix types, and their operators and methods its own operations. A <C>Quat</C> is a record of four
        parts there:
      </P>
      <Ex name="vec/gpu-fn" />
      <P>
        A vertex fn&apos;s <C>position</C>, and a fragment fn&apos;s color, may be a <C>Vec4&lt;Float32&gt;</C>, so a
        camera&apos;s <C>Mat4</C> places each corner with one <C>@</C>. See{" "}
        <Link href="/docs/speed/gpu/drawing-triangles/">Drawing triangles</Link>.
      </P>
      <P>Not yet: SIMD registers for <C>Vec4&lt;Float32&gt;</C> and <C>Mat4&lt;Float32&gt;</C> on the CPU.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.28 · std.vec" />
        <SrcRef file="tests/conformance/14-standard-library/vectors-matrices-and-quaternions.kav" label="tested" />
        <SrcRef file="tests/conformance/14-standard-library/vectors-of-float32.kav" label="Float32, tested" />
        <SrcRef file="tests/conformance/14-standard-library/three-by-three-matrices.kav" label="Mat3, tested" />
        <SrcRef file="tests/conformance/14-standard-library/whole-number-vectors.kav" label="whole numbers, tested" />
        <SrcRef file="tests/conformance/15-gpu/gpu-fn-uses-std-vec.kav" label="in gpu fns, tested" />
        <SrcRef file="stdlib/vec/vec.kav" label="vec.kav" />
        <SrcRef file="docs/decisions/0115-generic-math-and-std-vec.md" label="decision 0115" />
      </div>
    </>
  ),
};
