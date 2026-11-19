import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Fonts ----------
//
// Spec 14.31. Loading a real font reads a file, so the spec's example is
// shown, not run; the bad-font sample runs, as it needs no file.

const FILE = "14-standard-library/31-font.md";

export const fontDoc: LessonDoc = {
  lead: "std.font reads OpenType and TrueType fonts, shapes text into glyphs and their places, and draws glyphs into an atlas the GPU can draw text from at any size. A font is data, so it needs no capability.",
  simple: <>A font file holds the shape of every letter. Writing a word means picking each shape and placing it just right, joined where a script joins its letters. <C>std.font</C> does that, and draws the shapes into one picture that a game can draw text from.</>,
  minutes: 5,
  before: ["standard-library/images", "speed/gpu"],
  body: (
    <>
      <P>
        Load a font, shape a line in English and Hindi, and draw an atlas for the GPU. It reads a font file, so it is
        shown here, not run:
      </P>
      <CodeBlock code={specBlockWith(FILE, "14.31", "font.load(")} file="main.kav, from spec 14.31" />
      <ModuleTable file="31-font.md" number="14.31" first="Function or method" />
      <H2>Shaping</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Line</C> is <C>glyphs: [Glyph]</C> and <C>width: Float</C>, in pixels.</li>
        <li>A <C>Glyph</C> is <C>id</C>, its number in the font, and <C>x</C> and <C>y</C>, where its origin goes: from the start of the line, on the baseline, <C>y</C> growing downwards as on the screen.</li>
        <li>Shaping is HarfBuzz&apos;s: kerning, ligatures, and the joined and reordered forms of scripts such as Devanagari and Arabic.</li>
        <li>A glyph is not a letter: <C>&quot;fi&quot;</C> can be one glyph, and <C>&quot;क्ष&quot;</C> one glyph from three letters.</li>
        <li>The script and direction come from the text itself; right-to-left text is placed from right to left.</li>
        <li>A <C>Font</C> has <C>ascent</C>, <C>descent</C> (negative) and <C>line_gap</C> in ems, so lines are <C>(ascent - descent + line_gap) * size</C> pixels apart.</li>
      </ul>
      <H2>The atlas</H2>
      <P>
        An <C>Atlas</C> is an <C>image</C> (an <Link href="/docs/standard-library/images/">Image</Link>), its{" "}
        <C>size</C> and <C>spread</C>, and <C>boxes</C>, a dict from glyph number to <C>GlyphBox</C>: where the
        glyph is in the image (<C>x</C>, <C>y</C>, <C>width</C>, <C>height</C>), and where the box goes from the
        glyph&apos;s origin (<C>left</C>, <C>top</C>). A glyph with nothing to draw, as a space, has no box.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each pixel holds a distance from the glyph&apos;s edge: 128 on the edge, up to 255 at <C>spread</C> pixels inside, down to 0 at <C>spread</C> pixels outside.</li>
        <li><C>spread</C> is an eighth of <C>size</C>, at least 2.</li>
        <li>A shader that draws the pixels above 128 draws the glyph sharp at any scale. So one atlas made at 48 draws text at 32 or 96: multiply the boxes by the size wanted over <C>atlas.size</C>.</li>
        <li>Its pixels are what <C>gpu.texture</C> takes. See <Link href="/docs/speed/gpu/drawing-triangles/">Drawing triangles</Link>.</li>
      </ul>
      <H2>When it fails</H2>
      <P>Bytes that aren&apos;t a font file, or a file that is cut or damaged, give <C>BadFont</C>:</P>
      <Ex name="font/not-a-font" />
      <P>A <C>size</C> outside 1 to 1,000 is a panic, and so is an atlas wider or taller than 4,096 pixels.</P>
      <P>
        Not yet: breaking text into lines, mixed directions in one line, falling back to another font for a missing
        glyph, colour emoji, and variable fonts&apos; axes.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.31 · std.font" />
        <SrcRef file="tools/cli/tests/fonts.rs" line={47} symbol="text_is_shaped_and_drawn_into_an_atlas" label="shaping and atlas, tested" />
        <SrcRef file="tests/conformance/14-standard-library/bytes-that-are-not-a-font-are-bad-font.kav" label="BadFont, tested" />
        <SrcRef file="stdlib/font/font.kav" label="font.kav" />
        <SrcRef file="docs/decisions/0121-fonts-through-rustybuzz.md" label="decision 0121" />
      </div>
    </>
  ),
};
