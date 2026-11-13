import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Compression ----------

const SPEC = "docs/spec/14-standard-library/21-compress.md";

export const compressDoc: LessonDoc = {
  lead: "std.compress makes bytes smaller and whole again, in gzip, deflate, zstd, brotli and lz4: the formats web servers, packages and games exchange.",
  simple: <>Like vacuum-packing clothes for a trip: the same clothes take less room in the bag, and come out the same at the other end. Text that repeats itself packs very small.</>,
  minutes: 4,
  before: ["standard-library/bytes"],
  body: (
    <>
      <Ex name="compress/round-trip" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>encode</C> packs <C>Bytes</C> in a <C>Format</C>; <C>decode</C> gives back the same bytes.</li>
        <li>It needs no capability: it changes nothing outside the program.</li>
        <li>The formats come from Rust libraries, built into the runtime.</li>
      </ul>
      <H2>Levels</H2>
      <P>
        Each format has its own levels: a higher one is smaller and slower. Leave <C>level</C> out for the format&apos;s
        usual one:
      </P>
      <ModuleTable file="21-compress.md" number="14.21" first="`Format`" />
      <P>
        A level outside its format&apos;s range is a mistake in the program, not in the data, so it breaks{" "}
        <C>encode</C>&apos;s contract and panics. <C>compress.level_fits</C> checks one first:
      </P>
      <Ex name="compress/level.panics" />
      <H2>The format a header names</H2>
      <P>
        HTTP names formats in headers such as <C>accept-encoding</C>. <C>compress.from_name</C> reads such a name, in
        any case, and <C>format.name()</C> writes one:
      </P>
      <Ex name="compress/header" />
      <H2>Decoding stops at most</H2>
      <P>
        A small input can expand to gigabytes, a &quot;zip bomb&quot;. So <C>decode</C> gives at most <C>most</C>{" "}
        bytes, 64 MB unless the call says otherwise, and fails with <C>TooLarge</C> past it instead of filling memory.
        Data that isn&apos;t in the format, or was cut short, fails with <C>Corrupt</C>:
      </P>
      <Ex name="compress/most" />
      <ModuleTable file="21-compress.md" number="14.21" first="`CompressError`" />
      <H2>Every function</H2>
      <ModuleTable file="21-compress.md" number="14.21" first="Function" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 14.21 · std.compress" />
        <SrcRef file="tests/conformance/14-standard-library/compress-gives-back-what-it-took.kav" label="tested" />
        <SrcRef file="tests/conformance/14-standard-library/compress-decoding-stops-at-most.kav" label="most, tested" />
        <SrcRef file="tests/conformance/14-standard-library/compress-levels-outside-the-range-panic.kav" label="levels, tested" />
        <SrcRef file="docs/decisions/0103-std-compress.md" label="decision 0103" />
      </div>
    </>
  ),
};
