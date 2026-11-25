import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P, SpecCode } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Fonts ----------
//
// Spec 14.31. Loading a real font reads a file, so the spec's example is
// shown, not run; the bad-font sample runs, as it needs no file.

const FILE = "14-standard-library/31-font.md";

export const fontDoc: LessonDoc = {
  lead: "std.font reads OpenType and TrueType fonts, shapes text into glyphs and their places, and draws glyphs into an atlas the GPU draws text from: at any size, or sharp at one small size. A font is data, so it needs no capability.",
  simple: <>A font file holds the shape of every letter. Writing a word means picking each shape and placing it just right, joined where a script joins its letters. <C>std.font</C> does that, and draws the shapes into one picture that a game can draw text from.</>,
  minutes: 5,
  before: ["standard-library/images", "speed/gpu"],
  body: (
    <>
      <P>
        Load a font, shape a line in English and Hindi, and draw an atlas for the GPU. It reads a font file, so it is
        shown here, not run:
      </P>
      <SpecCode file={FILE} number="14.31" text={"font.load("} name="main.kav" />
      <ModuleTable file="31-font.md" number="14.31" first="Function or method" />
      <H2>Shaping</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Line</C> is <C>glyphs: [Glyph]</C> and <C>width: Float</C>, in pixels.</li>
        <li>A <C>Glyph</C> is <C>id</C>, its number in the font; <C>x</C> and <C>y</C>, where its origin goes: from the start of the line, on the baseline, <C>y</C> growing downwards as on the screen; <C>cluster</C>, the character of the text it comes from; and <C>advance</C>, how far it moves the pen to the right, in pixels.</li>
        <li>Shaping is HarfBuzz&apos;s: kerning, ligatures, and the joined and reordered forms of scripts such as Devanagari and Arabic.</li>
        <li>A glyph is not a letter: <C>&quot;fi&quot;</C> can be one glyph, and <C>&quot;क्ष&quot;</C> one glyph from three letters.</li>
        <li>The script and direction come from the text itself; right-to-left text is placed from right to left.</li>
        <li>A <C>Font</C> has <C>ascent</C>, <C>descent</C> (negative) and <C>line_gap</C> in ems, so lines are <C>(ascent - descent + line_gap) * size</C> pixels apart.</li>
      </ul>
      <H2>Clusters: from a glyph back to the text</H2>
      <P>
        A cluster is the smallest part of the text that maps to whole glyphs: <C>&quot;fi&quot;</C> drawn as one glyph
        is one cluster of two characters. Each glyph has its cluster&apos;s first character, counted from 0 as{" "}
        <C>s.len()</C> counts. So a click on a glyph finds its place in the text, and a caret goes between clusters,
        never inside one. This sample uses a tiny font of three glyphs, carried in the code, so it needs no file:
      </P>
      <Ex name="font/clusters" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>With a font that joins <C>&quot;ffi&quot;</C> into one glyph, <C>&quot;office&quot;</C> gives clusters 0, 1, 4 and 5.</li>
        <li>Left-to-right text gives clusters in rising order. Right-to-left text gives them falling, as its glyphs are placed from the right.</li>
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
      <H2>Exact atlases, for small text</H2>
      <P>
        Distances blur at 10 to 14 pixels, where most of a user interface&apos;s text is. <C>exact: true</C> draws
        each glyph for one size only: each pixel holds how much of it the glyph covers, from 0 to 255, from 64 points
        in the pixel. Drawn at that size on whole pixels, the glyphs stay sharp:
      </P>
      <Ex name="font/exact-atlas" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Round the glyph&apos;s origin to a whole pixel before adding <C>left</C> and <C>top</C>.</li>
        <li>Only the first of the four parts is needed: a <Link href="/docs/standard-library/gpu-screens/drawing-for-user-interfaces/">one-channel texture</Link> holds it in a quarter of the memory.</li>
        <li><strong className="text-ink">Colour glyphs.</strong> In an exact atlas, a glyph the font gives in colour, as an emoji of a font with a <C>COLR</C> table, is drawn in its colours, and its box&apos;s <C>color</C> is true. A shader draws such a box&apos;s pixels as they are, and the others in the text&apos;s colour. In a distance atlas every glyph is its outline.</li>
      </ul>
      <H2>Adding glyphs as text is typed</H2>
      <P>
        <C>atlas.add(f, text)</C> draws each glyph of <C>text</C> the atlas lacks, as the atlas was drawn, in new rows
        below its image. Every old box stays where it was, so a texture made from it needs only the new rows sent
        again, with <C>gpu.update</C>:
      </P>
      <Ex name="font/atlas-add" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The image grows taller. It stays as wide, or grows as wide as the widest new glyph.</li>
        <li>An atlas holds one font&apos;s glyphs, as the numbers are that font&apos;s.</li>
        <li>An atlas taller than 4,096 pixels is a panic: make a new one then.</li>
      </ul>
      <H2>When it fails</H2>
      <P>Bytes that aren&apos;t a font file, or a file that is cut or damaged, give <C>BadFont</C>:</P>
      <Ex name="font/not-a-font" />
      <P>A <C>size</C> outside 1 to 1,000 is a panic, and so is an atlas wider or taller than 4,096 pixels.</P>
      <P>
        To use the fonts the machine has, and find one for a script another font lacks, see{" "}
        <Link href="/docs/standard-library/fonts/installed-fonts/">Installed fonts</Link>.
      </P>
      <P>
        Not yet: breaking text into lines, mixed directions in one line, variable fonts&apos; axes, and OpenType
        features the program chooses, such as small capitals.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.31 · std.font" />
        <SrcRef file="tools/cli/tests/fonts.rs" line={47} symbol="text_is_shaped_and_drawn_into_an_atlas" label="shaping and atlas, tested" />
        <SrcRef file="tests/conformance/14-standard-library/bytes-that-are-not-a-font-are-bad-font.kav" label="BadFont, tested" />
        <SrcRef file="stdlib/font/font.kav" label="font.kav" />
        <SrcRef file="tests/conformance/14-standard-library/font-colour-glyph-in-an-exact-atlas.kav" label="colour glyphs, tested" />
        <SrcRef file="docs/decisions/0121-fonts-through-rustybuzz.md" label="decision 0121" />
      </div>
    </>
  ),
};
