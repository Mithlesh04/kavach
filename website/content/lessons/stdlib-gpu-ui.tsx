import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P, SpecCode } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- GPU Screens › Drawing for user interfaces ----------
//
// Spec 14.24.1: clipping, masks, premultiplied blending, 4-sample smooth
// edges, canvases and texture updates. A canvas draws off the screen and
// opens no window, so these samples run on the GPU of the machine that
// builds the site and read the pixels back. With no GPU, each says so.

const FILE = "14-standard-library/24-gpu.md";
const TESTS = "tests/conformance/14-standard-library";

export const gpuUiDoc: LessonDoc = {
  lead: "A user interface draws thousands of small shapes a frame. std.gpu clips them to a box, masks them to a rounded shape, smooths their edges, draws into canvases off the screen, and changes part of a texture as a glyph atlas grows.",
  simple: <>Think of a scrolled list in an app: the rows must not spill out of their box, the box has round corners, and text must look sharp. These are the tools a UI library builds that on.</>,
  minutes: 7,
  before: ["standard-library/gpu-screens"],
  body: (
    <>
      <P>
        These are building blocks for a UI library, not a UI library. A frame keeps a state that every draw after it
        follows: a clip, a mask and a way of blending. A panel clips to its scrolled box and draws its text only inside
        its rounded shape:
      </P>
      <SpecCode file={FILE} number="14.24.1" text={"fn panel(frame: Frame, ui: Ui)"} />
      <H2>Canvases: drawing off the screen</H2>
      <P>
        A canvas is a picture off the screen. You draw into it as into a window, then read its pixels back or draw
        from it as a texture. It opens no window, so it works where there is no screen, given a GPU. Here a 4 × 4
        canvas is cleared red and read back:
      </P>
      <Ex name="gpu/canvas" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A new canvas is transparent black: all four parts of every pixel are 0.</li>
        <li><C>canvas.pixels()</C> gives four bytes a pixel (red, green, blue, opacity), row by row from the top, as the last frame shown left them. It waits for the GPU.</li>
        <li>A canvas&apos;s frame is a <C>Frame</C> like a screen&apos;s, with every method below. It doesn&apos;t wait for the screen&apos;s refresh.</li>
        <li>It needs <C>Window</C>, as all drawing does, though it opens no window.</li>
      </ul>
      <ModuleTable file={FILE} number="14.24.1" first="Function or method" />
      <P>A draw into a canvas can&apos;t read that canvas&apos;s own texture: a picture can&apos;t be read while it is drawn.</P>
      <Ex name="gpu/canvas-reads-itself.panics" />
      <H2>How a frame draws</H2>
      <ModuleTable file={FILE} number="14.24.1" first="Method" />
      <P>
        <strong className="text-ink">Clipping.</strong> <C>set_clip</C> keeps draws inside a rectangle, in pixels from
        the frame&apos;s top left. Here a white square over the whole canvas changes only its left half, until{" "}
        <C>clear_clip</C>:
      </P>
      <Ex name="gpu/clip" />
      <P>
        <strong className="text-ink">Masks.</strong> For a shape that isn&apos;t a rectangle, such as a box with
        rounded corners, a draw with <C>Mask.Write</C> marks the pixels it covers and colours none. After that,{" "}
        <C>Mask.Inside</C> draws only on marked pixels, and <C>Mask.Outside</C> only on the others. Here the left half
        is marked, then red is drawn inside and blue outside:
      </P>
      <Ex name="gpu/mask" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A pixel is marked or not: marking it twice marks it once. Masks don&apos;t nest.</li>
        <li>A rectangle is cheaper with <C>set_clip</C> than with a mask.</li>
        <li><C>set_clip</C> with a negative number is a panic. A rectangle past the frame&apos;s edges is cut to them.</li>
        <li>A new frame starts with no clip, no mask and plain blending.</li>
        <li>A pipeline&apos;s first draw with a mask or premultiplied blending compiles that form of it. Draw it once while the program loads, so no frame waits for it.</li>
      </ul>
      <P>
        <strong className="text-ink">Premultiplied blending.</strong> With <C>set_premultiplied(true)</C>, a fragment
        fn&apos;s red, green and blue are taken as already multiplied by its opacity, as pictures with soft edges keep
        them. This avoids dark fringes around text and icons.
      </P>
      <H2>Smooth edges</H2>
      <P>
        With <C>samples: 4</C>, on <C>gpu.screen</C> or <C>gpu.canvas</C>, each pixel is drawn from 4 samples and
        blended when shown. A pixel that a triangle&apos;s edge crosses is then partly coloured, so the edge looks
        smooth, not stepped:
      </P>
      <Ex name="gpu/smooth" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It costs the GPU about four times the memory for the picture.</li>
        <li><C>samples</C> is 1 or 4; any other number is a panic.</li>
      </ul>
      <H2>Textures for text and pictures</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">One channel.</strong> <C>channels: 1</C> takes one byte a pixel, a quarter of the memory. <C>sample</C> gives the value as red: <C>(value, 0.0, 0.0, 1.0)</C>. It is a value, not a colour, as a glyph atlas&apos;s coverage is (see <Link href="/docs/standard-library/fonts/">Fonts</Link>).</li>
        <li><strong className="text-ink">Nearest pixel.</strong> <C>nearest: true</C> samples the pixel under the spot instead of blending the four nearest: for pixel art, and for a glyph atlas drawn at its own size. The textures of one draw are all nearest, or none is.</li>
        <li><strong className="text-ink">Changing part of a texture.</strong> <C>gpu.update(t, x, y, width, height, pixels)</C> changes only that part, on the GPU and in <C>t</C>, so <C>sample</C> on the CPU sees it too.</li>
      </ul>
      <P>Here one pixel of a 2 × 2 one-channel texture is set, and the others stay:</P>
      <Ex name="gpu/texture-update" />
      <P>
        <C>t</C> is an <Link href="/docs/functions/inout-parameters/">inout</Link> parameter, so it is a <C>let</C>. A part past
        the texture&apos;s edges, the wrong number of bytes, or a texture only the GPU reads is a panic. A glyph atlas
        that grows sends only its new rows, and makes a new texture only when it gets taller:
      </P>
      <SpecCode file={FILE} number="14.24.1" text={"atlas = atlas.add(sans, typed)"} />
      <Note>
        Not built yet, and listed as open questions in the spec: lines and points as well as triangles, other kinds
        of blending, and choosing the present mode (vsync off) or the colour format.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.24.1 · drawing for user interfaces" />
        <SrcRef file="stdlib/gpu/gpu.kav" line={49} symbol="set_clip" />
        <SrcRef file="stdlib/gpu/gpu.kav" line={120} symbol="Canvas" />
        <SrcRef file="stdlib/gpu/gpu.kav" line={250} symbol="update" />
      </div>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`${TESTS}/gpu-clip-keeps-draws-inside.kav`} label="clip, tested" />
        <SrcRef file={`${TESTS}/gpu-mask-limits-draws.kav`} label="masks, tested" />
        <SrcRef file={`${TESTS}/gpu-premultiplied-blending.kav`} label="premultiplied, tested" />
        <SrcRef file={`${TESTS}/gpu-four-samples-smooth-an-edge.kav`} label="4 samples, tested" />
        <SrcRef file={`${TESTS}/gpu-update-changes-part-of-a-texture.kav`} label="update, tested" />
      </div>
    </>
  ),
};
