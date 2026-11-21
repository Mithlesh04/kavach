import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { TENSOR_SPEC, TENSOR_SRC, TENSOR_TESTS, tensorTable } from "./stdlib-tensor";

// ---------- Tensors: linear algebra and the FFT ----------
//
// Spec 14.33's "Linear algebra" and "The FFT". `@` itself is on the
// overview (stdlib-tensor.tsx); norm is also on Reductions.

export const tensorLinalgDoc: LessonDoc = {
  lead: "a.inv(), a.solve(b), a.det(), a.eig() and a.svd() work on a matrix of floats: a tensor of two dimensions. Each is one fixed algorithm, so its result has the same bits on every machine.",
  simple: <>A matrix can stand for a set of equations, such as 2x + y = 3 and x + 3y = 5. solve finds the x and y that make them all true, as you would on paper, only for thousands of them at once.</>,
  minutes: 6,
  before: ["standard-library/tensors"],
  body: (
    <>
      <Ex name="tensor/linalg" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>det</C> is the determinant, <C>inv</C> the inverse and <C>solve(b)</C> the <C>x</C> with <C>a @ x == b</C>. They use LU decomposition with partial pivoting.</li>
        <li><C>solve</C> doesn&apos;t work out the inverse first. Its <C>b</C> has 1 or 2 dimensions, and <C>x</C> has <C>b</C>&apos;s shape.</li>
        <li>Floats round, so a result is right up to rounding: <C>a @ x</C> gives <C>4.999999999999999</C> where the true answer is 5. Compare floats with a tolerance, as the samples below do.</li>
        <li><C>norm()</C> is the square root of the sum of the squares; <Link href="/docs/standard-library/tensors/reductions/">Reductions</Link> shows it along an axis.</li>
      </ul>
      <H2>A matrix with no inverse</H2>
      <P>
        <C>inv</C> and <C>solve</C> can fail, with <C>tensor.Singular</C>, so each call gets a <C>catch</C>. The error
        holds the matrix&apos;s <C>shape</C>. It happens when a pivot is exactly 0, as here, where one row is the
        other doubled; the <C>det</C> of such a matrix is <C>0.0</C>:
      </P>
      <Ex name="tensor/singular" />
      <H2>Eigenvalues</H2>
      <P>
        <C>a.eig()</C> takes a symmetric matrix and gives an <C>Eig</C> record: <C>values</C>, smallest first, and{" "}
        <C>vectors</C>, one column for each value:
      </P>
      <Ex name="tensor/eig" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It uses the Jacobi method, in a fixed order, so its result has the same bits everywhere.</li>
        <li>A vector times -1 is still a vector of the same value. So each is turned so that its number of largest size is positive: the result is the same everywhere, not only up to a sign.</li>
      </ul>
      <P>
        <C>eig</C> checks that <C>a[i, j] == a[j, i]</C> for every pair, exactly. A matrix that isn&apos;t symmetric
        can have complex eigenvalues, which <C>eig</C> doesn&apos;t give yet, so it is a panic naming the first pair that differs:
      </P>
      <Ex name="tensor/eig-symmetric.panics" />
      <H2>The singular value decomposition</H2>
      <P>
        <C>a.svd()</C> takes any matrix and gives an <C>Svd</C> record: <C>u</C>, <C>s</C> (largest first) and{" "}
        <C>vt</C>, with <C>a == u @ diag(s) @ vt</C> up to rounding. <C>u * s</C> scales <C>u</C>&apos;s columns,
        as <C>u @ diag(s)</C> does:
      </P>
      <Ex name="tensor/svd" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It is the reduced form: for an <C>[m, n]</C> matrix and <C>k = min(m, n)</C>, <C>u</C> is <C>[m, k]</C>, <C>s</C> is <C>[k]</C> and <C>vt</C> is <C>[k, n]</C>.</li>
        <li>It uses the one-sided Jacobi method. This matrix&apos;s singular values are 5 and 3; the second comes out as <C>2.9999999999999996</C>, the same on every machine.</li>
        <li>Each column of <C>u</C> is turned so that its number of largest size is positive, and the row of <C>vt</C> that goes with it is turned with it.</li>
      </ul>
      <H2>What is refused</H2>
      <P>A matrix that isn&apos;t square, where one is needed, is a panic naming its shape:</P>
      <Ex name="tensor/square.panics" />
      <P>Linear algebra needs <C>T: Floating</C>. A tensor of whole numbers is a compile error; convert it first, with <C>Tensor&lt;Float&gt;(t)</C>:</P>
      <Ex name="tensor/linalg-int.fails" />
      <H2>Every call</H2>
      {tensorTable("Linear algebra")}
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · linear algebra" />
        <SrcRef file={`${TENSOR_SRC}/linalg.kav`} line={16} symbol="fn inv" />
        <SrcRef file={`${TENSOR_SRC}/linalg.kav`} line={32} symbol="fn solve" />
        <SrcRef file={`${TENSOR_SRC}/linalg.kav`} line={47} symbol="fn det" />
        <SrcRef file={`${TENSOR_SRC}/decompose.kav`} line={18} symbol="fn eig" />
        <SrcRef file={`${TENSOR_SRC}/decompose.kav`} line={44} symbol="fn svd" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={191} symbol="type Singular" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={198} symbol="type Eig" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={205} symbol="type Svd" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-inv-gives-the-inverse.kav`} label="inv, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-solve-without-the-inverse.kav`} label="solve, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-solve-of-other-sizes-panics.kav`} label="solve's sizes, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-det-of-a-known-matrix.kav`} label="det, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-singular-matrix-fails.kav`} label="Singular, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-eig-of-a-symmetric-matrix.kav`} label="eig, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-eig-of-a-non-symmetric-matrix-panics.kav`} label="symmetric only, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-svd-gives-the-matrix-back.kav`} label="svd, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-matrix-not-square-panics.kav`} label="square only, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-linear-algebra-needs-floating.kav`} label="floats only, tested" />
      </div>
    </>
  ),
};

export const tensorFftDoc: LessonDoc = {
  lead: "t.fft() gives the discrete Fourier transform of a tensor of floats, as a Spectrum: its real parts and its imaginary parts. s.ifft() gives the signal back. Any length works.",
  simple: <>A sound is many waves added together. The FFT finds how much of each wave is in it, from the slowest to the fastest, and ifft adds them back into the sound.</>,
  minutes: 4,
  before: ["standard-library/tensors"],
  body: (
    <>
      <Ex name="tensor/fft" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Spectrum</C> is complex numbers as two tensors of one shape: <C>re</C>, the real parts, and <C>im</C>, the imaginary parts.</li>
        <li><C>s.abs()</C> is each number&apos;s size, <C>sqrt(re² + im²)</C>.</li>
        <li>As in NumPy, <C>fft</C> doesn&apos;t scale, and <C>ifft</C> divides by the length.</li>
        <li><C>T</C> is <C>Float</C> or <C>Float32</C>.</li>
      </ul>
      <H2>Any length</H2>
      <P>
        A length that is a power of two uses the radix-2 algorithm. Any other length uses Bluestein&apos;s, which
        works through radix-2 transforms of a longer power of two:
      </P>
      <Ex name="tensor/fft-length" />
      <P>
        The angles come from <C>std.math</C>&apos;s <C>sin</C> and <C>cos</C>, so the result has
        the same bits on every machine.
      </P>
      <H2>Along an axis</H2>
      <P>Without <C>axis:</C>, each transform runs along the last axis: each row of a matrix. <C>axis: 0</C> runs down each column:</P>
      <Ex name="tensor/fft-rows" />
      <H2>What is refused</H2>
      <P>A <C>Spectrum</C> made by hand must have <C>re</C> and <C>im</C> of one shape. <C>fft</C>, <C>ifft</C> or <C>abs</C> of two shapes is a panic naming both:</P>
      <Ex name="tensor/spectrum.panics" />
      <H2>Every call</H2>
      {tensorTable("The FFT")}
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${TENSOR_SPEC}`} label="spec 14.33 · the FFT" />
        <SrcRef file={`${TENSOR_SRC}/fft.kav`} line={20} symbol="fn fft" />
        <SrcRef file={`${TENSOR_SRC}/fft.kav`} line={33} symbol="fn ifft" />
        <SrcRef file={`${TENSOR_SRC}/fft.kav`} line={38} symbol="fn abs" />
        <SrcRef file={`${TENSOR_SRC}/fft.kav`} line={184} symbol="fn bluestein" />
        <SrcRef file={`${TENSOR_SRC}/tensor.kav`} line={214} symbol="type Spectrum" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-fft-of-a-known-signal.kav`} label="fft, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-ifft-gives-the-signal-back.kav`} label="ifft, tested" />
        <SrcRef file={`${TENSOR_TESTS}/tensor-spectrum-of-two-shapes-panics.kav`} label="one shape, tested" />
      </div>
    </>
  ),
};
