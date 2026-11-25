import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, P, SpecCode } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Fonts › Installed fonts ----------
//
// Spec 14.31.1: font.installed(), font.fallback(text) and installed.load().
// What fonts a machine has is about that machine, so the sample prints
// only true or false, never a family or a path.

const FILE = "14-standard-library/31-font.md";

export const fontInstalledDoc: LessonDoc = {
  lead: "A program can use the fonts the machine has instead of shipping each one, and find a fallback: an installed font with the glyphs another font lacks. Kavach shapes and draws them itself, so text looks the same on every system.",
  simple: <>Your app ships one font for English. Someone types Hindi, and that font has no Hindi letters. Instead of boxes, the app asks which installed font has them, and draws those letters with it.</>,
  minutes: 3,
  before: ["standard-library/fonts"],
  body: (
    <>
      <SpecCode file={FILE} number="14.31.1" text={"fn fonts()"} />
      <ModuleTable file={FILE} number="14.31.1" first="Function or method" />
      <P>
        Here a program checks that it finds fonts, and a font for Hindi. Which fonts a machine has is about that
        machine, so the sample prints only true or false:
      </P>
      <Ex name="font/installed" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An <C>InstalledFont</C> is <C>family</C> (<C>Segoe UI</C>), <C>style</C> (<C>Regular</C>, <C>Bold</C>), from the font&apos;s own names in English where it has them, and <C>path</C>, its file.</li>
        <li>The system is only asked where its fonts are: on Windows, the <C>Fonts</C> folders of Windows and of the user; on Linux, <C>/usr/share/fonts</C>, <C>/usr/local/share/fonts</C>, <C>~/.local/share/fonts</C> and <C>~/.fonts</C>; on macOS, <C>/System/Library/Fonts</C>, <C>/Library/Fonts</C> and <C>~/Library/Fonts</C>. They are looked for once a run.</li>
        <li>A file holding several fonts (<C>.ttc</C>) gives its first, as <C>font.load</C> reads it.</li>
        <li><C>fallback</C> tries first the families the system&apos;s own text falls back to (on Windows: Segoe UI, Segoe UI Symbol, Segoe UI Emoji, Nirmala UI, Microsoft YaHei and others), then the rest by family; each family&apos;s regular style before its others.</li>
        <li>Spaces and control characters don&apos;t need a glyph.</li>
        <li>A machine with no fonts, as a container, gives <C>[]</C> and <C>none</C>.</li>
      </ul>
      <P>
        Listing the fonts says something about the machine, as its other details do (see{" "}
        <Link href="/docs/standard-library/the-machine/">The Machine</Link>), so it needs <C>System</C>.{" "}
        <C>installed.load()</C> reads only an installed font&apos;s file, so it needs no <C>Fs.read</C>:
      </P>
      <Ex name="font/installed-system.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.31.1 · installed fonts" />
        <SrcRef file="stdlib/font/font.kav" line={133} symbol="installed" />
        <SrcRef file="stdlib/font/font.kav" line={144} symbol="fallback" />
        <SrcRef file="tests/conformance/14-standard-library/font-installed-fonts-and-fallback.kav" label="installed and fallback, tested" />
        <SrcRef file="docs/decisions/0121-fonts-through-rustybuzz.md" label="decision 0121" />
      </div>
    </>
  ),
};
