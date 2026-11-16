import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Images ----------
//
// Spec 14.29. An image is data and needs no capability, so the samples
// make one, write it as PNG and read it back, all in memory.

const FILE = "14-standard-library/29-image.md";

export const imageDoc: LessonDoc = {
  lead: "std.image reads PNG and JPEG files into an Image, writes an Image as PNG, and gives its pixels to the GPU as they are. An image is data, so it needs no capability.",
  simple: <>A picture on a screen is a grid of tiny colored dots, pixels. An <C>Image</C> is that grid as numbers: how wide, how high, and four numbers for each dot, its red, green, blue and how see-through it is.</>,
  minutes: 4,
  before: ["standard-library/bytes", "standard-library/files"],
  body: (
    <>
      <P>
        Make a picture of two pixels, write it as a PNG file&apos;s bytes, and read it back. Nothing touches the disk,
        so it runs here:
      </P>
      <Ex name="image/round-trip" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An <C>Image</C> is a record: <C>width</C>, <C>height</C> and <C>pixels</C>, a <C>[UInt8]</C>.</li>
        <li>Each pixel is four numbers from 0 to 255: red, green, blue and opacity, row by row from the top.</li>
        <li><C>img.pixel(x, y)</C> gives one pixel&apos;s four numbers; <C>x</C> 0 is the left, <C>y</C> 0 the top.</li>
        <li>PNG loses nothing, so the picture read back is equal to the one written.</li>
      </ul>
      <H2>Files and the GPU</H2>
      <P>
        Reading a file needs <C>Fs.read</C>, as always; the picture itself needs nothing. Its pixels are what{" "}
        <C>gpu.texture</C> takes, so a picture goes to the GPU as it is. Shown, not run, as it reads a file:
      </P>
      <CodeBlock code={specBlockWith(FILE, "14.29", "image.decode(")} file="from spec 14.29" />
      <P>
        Every image is read as 8 bits a part, with opacity: a grey image&apos;s three colors are the same, and an
        image without opacity is fully opaque. See{" "}
        <Link href="/docs/speed/gpu/drawing-triangles/">Drawing triangles</Link> for textures.
      </P>
      <H2>When it fails</H2>
      <P>Bytes that aren&apos;t a PNG or JPEG file, or a file that is cut or damaged, give <C>BadImage</C>:</P>
      <Ex name="image/not-an-image" />
      <P><C>image.new</C> checks its pixels: <C>width * height * 4</C> numbers, no more and no fewer:</P>
      <Ex name="image/wrong-count.panics" />
      <H2>Every function and method</H2>
      <ModuleTable file="29-image.md" number="14.29" first="Function or method" />
      <P>
        Not yet: GIF and WebP, and resizing, which can be packages written over <C>Image</C>. Compressed GPU textures
        come from KTX2 files: see <Link href="/docs/speed/gpu/drawing-triangles/">Drawing triangles</Link>. Fonts are <Link href="/docs/standard-library/fonts/">std.font</Link>: its atlas is an{" "}
        <C>Image</C>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.29 · std.image" />
        <SrcRef file="tests/conformance/14-standard-library/images-read-and-write-png.kav" label="tested" />
        <SrcRef file="stdlib/image/image.kav" label="image.kav" />
        <SrcRef file="docs/decisions/0117-images-png-and-jpeg.md" label="decision 0117" />
      </div>
    </>
  ),
};
