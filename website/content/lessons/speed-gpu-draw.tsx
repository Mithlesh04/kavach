import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";

// ---------- GPU › Drawing triangles ----------
//
// Spec 15.3. A drawing program opens a window, so this site shows the
// spec's example and doesn't run it; the same gpu fns are run on the
// CPU instead, and the compile errors are checked.

export const gpuDrawDoc: LessonDoc = {
  lead: "A gpu vertex fn places each corner of a triangle, a gpu fragment fn colors each pixel inside it, and gpu.pipeline joins the two for frame.draw.",
  simple: <>A picture on a screen is made of many small triangles. For each corner, the GPU asks one function where it goes. For each pixel inside, it asks another what color it is. You write both functions in Kavach, and the GPU runs them for millions of pixels at once.</>,
  minutes: 6,
  before: ["speed/gpu/gpu-fn", "standard-library/gpu-screens"],
  body: (
    <>
      <P>
        This is the spec&apos;s example: dots of a swarm, each drawn as a square, under a camera. It draws into a
        window, so it is shown here, not run:
      </P>
      <CodeBlock code={specBlockWith("15-gpu.md", "15.3", "gpu vertex fn")} file="from spec 15.3" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          A <C>gpu vertex fn</C> takes the vertex, then the instance and the value shared by the whole draw, if the
          draw has them, in that order. It gives a record whose <C>position</C> is where the corner lands: x and y from
          -1 to 1 across the window, then depth and <C>w</C>.
        </li>
        <li>Its other fields go to the fragment fn, blended between the triangle&apos;s corners.</li>
        <li>A <C>gpu fragment fn</C> gives the pixel&apos;s color: red, green, blue and opacity, each from 0 to 1.</li>
        <li>
          <C>frame.draw(dots, square, instances: swarm, shared: camera)</C> draws the square once for each dot. A
          draw paints over what the frame already holds.
        </li>
        <li>A vertex, an instance or a shared value is a record of 1 to 8 fields, each a <C>Float32</C>, <C>Int32</C> or <C>UInt32</C>.</li>
        <li>
          A list a GPU <Link href="/docs/speed/gpu/map-on-the-gpu/">map</Link> made, still in GPU memory, is drawn from
          there with nothing copied, as vertices or instances. A swarm moved by a <C>map</C> each frame is drawn where it
          is, and never comes back to the CPU.
        </li>
      </ul>
      <H2>The same fns on the CPU</H2>
      <P>
        A vertex or fragment fn is an ordinary function on the CPU too. So you can call it with one corner and see
        what the GPU would get, in a test or a <C>print</C>, with no window:
      </P>
      <Ex name="gpu/draw-on-cpu" />
      <H2>Mistakes caught when compiling</H2>
      <P>
        Every part of a draw is checked before the program runs. A vertex fn must give a <C>position</C>:
      </P>
      <Ex name="gpu/draw-no-position.fails" />
      <P><C>gpu.pipeline</C> takes a <C>gpu vertex fn</C> and a <C>gpu fragment fn</C>, not plain functions:</P>
      <Ex name="gpu/draw-plain-fn.fails" />
      <P>
        A draw gives what the vertex fn takes. Here <C>place</C> takes a <C>Camera</C> as its second parameter, so the
        draw must give it as <C>instances:</C>:
      </P>
      <Ex name="gpu/draw-no-instances.fails" />
      <Note>
        A draw can&apos;t stop the program halfway through a frame. So in a vertex or fragment fn, an overflow or a
        division by zero gives a wrong value for that corner or pixel, not a panic. On the CPU, the same fn panics as
        any other.
      </Note>
      <P>
        A pipeline is compiled for the GPU when it is made, or when a screen opens, never in the middle of a frame.
        The window, the screen and the frame are on{" "}
        <Link href="/docs/standard-library/gpu-screens/">GPU Screens</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/15-gpu.md" label="spec 15.3 · drawing" />
        <SrcRef file="docs/spec/14-standard-library/24-gpu.md" label="spec 14.24 · std.gpu" />
        <SrcRef file="tests/conformance/15-gpu/gpu-vertex-and-fragment-fns-run-on-the-cpu.kav" label="on the CPU, tested" />
        <SrcRef file="compiler/backend/wgsl/src/tests.rs" symbol="a_pipeline_draws_what_its_gpu_fns_say" line={230} label="real pixels, tested" />
        <SrcRef file="compiler/backend/wgsl/src/tests.rs" symbol="a_pipeline_compiles_before_its_first_draw" line={253} label="compiled before a draw, tested" />
        <SrcRef file="compiler/kir/src/gpu_stay.rs" symbol="a_list_drawn_is_not_fetched" line={381} label="drawn in place, tested" />
        <SrcRef file="docs/decisions/0106-drawing-on-the-gpu.md" label="decision 0106" />
      </div>
    </>
  ),
};
