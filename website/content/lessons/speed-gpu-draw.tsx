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
  lead: "A gpu vertex fn places each corner of a triangle, a gpu fragment fn colors each pixel inside it, and gpu.pipeline joins the two for frame.draw, with depth for 3D and textures for pictures.",
  simple: <>A picture on a screen is made of many small triangles. For each corner, the GPU asks one function where it goes. For each pixel inside, it asks another what color it is. You write both functions in Kavach, and the GPU runs them for millions of pixels at once.</>,
  minutes: 9,
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
          -1 to 1 across the window, then depth and <C>w</C>, as four <C>Float32</C>s or a{" "}
          <Link href="/docs/standard-library/vectors/">Vec4&lt;Float32&gt;</Link>.
        </li>
        <li>Its other fields go to the fragment fn, blended between the triangle&apos;s corners.</li>
        <li>A <C>gpu fragment fn</C> gives the pixel&apos;s color: red, green, blue and opacity, each from 0 to 1, as four <C>Float32</C>s or a <C>Vec4&lt;Float32&gt;</C>.</li>
        <li>
          <C>frame.draw(dots, square, instances: swarm, shared: camera)</C> draws the square once for each dot. A
          draw paints over what the frame already holds.
        </li>
        <li>
          Each three vertices make a triangle. <C>indices:</C> names them by their place instead, so triangles share
          corners: see <Link href="/docs/standard-library/gpu-screens/">GPU Screens</Link>.
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
      <H2>Depth: the nearest pixel wins</H2>
      <P>
        A pipeline made with <C>depth: true</C> draws a pixel only where it is nearer than what is already there. So a
        3D scene can be drawn in any order, and a near box still hides the wall behind it. Shown, not run:
      </P>
      <CodeBlock code={specBlockWith("15-gpu.md", "15.3", "depth: true")} file="from spec 15.3" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A pixel&apos;s depth is the third part of <C>position</C> divided by the fourth: 0 is nearest, 1 farthest. Only pixels from 0 to 1 are drawn.</li>
        <li>Each frame keeps one depth for each pixel. They start at 1, the farthest, and <C>frame.clear</C> sets them back to 1.</li>
        <li>A pipeline without depth, the default, neither looks at the depths nor changes them: it draws over what is there.</li>
      </ul>
      <P>
        <C>depth:</C> is part of the pipeline, which is compiled before it draws. So it is written <C>true</C> or{" "}
        <C>false</C>, never a value worked out while the program runs:
      </P>
      <Ex name="gpu/depth-written.fails" />
      <H2>Textures: drawing pictures</H2>
      <P>
        A <C>gpu fragment fn</C> can read pictures, <C>Texture</C>s, which it takes after its input.{" "}
        <C>t.sample(u, v)</C> gives the color of texture <C>t</C> at <C>u</C> (0 left, 1 right) and <C>v</C> (0 top, 1
        bottom). Shown, not run:
      </P>
      <CodeBlock code={specBlockWith("15-gpu.md", "15.3", "fn paint(")} file="from spec 15.3" />
      <P>
        <C>gpu.texture(width, height, pixels)</C> makes a texture from its pixels, four numbers each: red, green, blue
        and opacity, row by row from the top. <C>sample</C> works on the CPU too, with the same colors, so this runs
        with no window:
      </P>
      <Ex name="gpu/texture-sample" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A color from <C>sample</C> is blended between the four pixels nearest the spot. A spot past an edge takes the edge&apos;s pixels.</li>
        <li>A texture&apos;s colors are in sRGB, as image files and screens have them, so a texture drawn as it is shows its own pixels.</li>
        <li>A texture is 1 to 8192 pixels wide and high. Making one needs a GPU: with none, <C>gpu.texture</C> fails with a <C>GpuError</C>.</li>
        <li><C>sample</C> is the one call on a <C>Texture</C> a <C>gpu fn</C> can make, and only a <C>gpu fragment fn</C> takes textures.</li>
      </ul>
      <P>
        <C>frame.draw</C> gives one texture for each <C>Texture</C> the fragment fn takes, in order, as{" "}
        <C>textures:</C>. Leaving it out is a compile error; a list of another length is a panic:
      </P>
      <Ex name="gpu/draw-no-textures.fails" />
      <H2>Compressed textures</H2>
      <P>
        <C>gpu.compressed_texture(file)</C> makes a texture from a KTX2 file&apos;s bytes. The GPU reads its compressed
        blocks as they are, so it takes 4 to 8 times less of the GPU&apos;s memory than <C>gpu.texture</C>&apos;s
        pixels: what big game pictures need.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The blocks are BC1 to BC7, which desktop GPUs read, or ASTC 4×4, which phones read. Width and height are each a multiple of 4.</li>
        <li>Its mip levels, the smaller copies for far away, come with it.</li>
        <li>A GPU that can&apos;t read its blocks gives <C>Unavailable</C>.</li>
        <li>Only the GPU reads it, so <C>sample</C> on the CPU is a panic.</li>
        <li><C>texture.width</C> and <C>texture.height</C> give any texture&apos;s size in pixels.</li>
      </ul>
      <P>Bytes that aren&apos;t a KTX2 file are <C>BadTexture</C>, on any machine:</P>
      <Ex name="gpu/bad-ktx2" />
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
        <SrcRef file="compiler/backend/wgsl/src/tests.rs" symbol="a_pipeline_draws_what_its_gpu_fns_say" line={270} label="real pixels, tested" />
        <SrcRef file="compiler/backend/wgsl/src/tests.rs" symbol="a_pipeline_compiles_before_its_first_draw" line={293} label="compiled before a draw, tested" />
        <SrcRef file="compiler/kir/src/gpu_stay.rs" symbol="a_list_drawn_is_not_fetched" line={385} label="drawn in place, tested" />
        <SrcRef file="docs/decisions/0106-drawing-on-the-gpu.md" label="decision 0106" />
        <SrcRef file="tests/conformance/15-gpu/gpu-pipeline-depth-is-true-or-false.kav" label="depth, tested" />
        <SrcRef file="tests/conformance/15-gpu/gpu-fragment-fn-takes-textures-after-its-input.kav" label="textures, tested" />
        <SrcRef file="tests/conformance/15-gpu/frame-draw-gives-the-textures-its-fragment-fn-reads.kav" label="textures: given, tested" />
        <SrcRef file="tests/conformance/15-gpu/gpu-fn-reads-a-texture-only-by-sample.kav" label="only sample, tested" />
        <SrcRef file="tests/conformance/14-standard-library/a-bad-ktx2-file-is-bad-texture.kav" label="BadTexture, tested" />
        <SrcRef file="tools/cli/tests/textures.rs" symbol="a_texture_samples_on_the_cpu_as_on_the_gpu" label="sampled on the CPU, tested" />
      </div>
    </>
  ),
};
