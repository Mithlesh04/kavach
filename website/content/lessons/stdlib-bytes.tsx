import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { loadSample } from "@/lib/samples";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable, Table } from "./spec-table";

// ---------- Bytes ----------

/** One row per number type: each sample line's type and bytes, with what that line printed. */
function numberTypes() {
  const sample = loadSample("bytes/as-list-types");
  const printed = (sample.output ?? "").split("\n");
  const lines = sample.code.split("\n").filter((l) => l.includes(".as_list<"));
  if (lines.length !== printed.length) throw new Error("bytes/as-list-types: a line of output for each type");
  const kind = (t: string) =>
    t.startsWith("Float") ? "A number with a fraction" : t.startsWith("U") ? "A whole number, never negative" : "A whole number, which can be negative";
  const rows = lines.map((line, i) => {
    const found = line.match(/(Bytes\(\[(.*)\]\))\.as_list<(\w+)>/);
    if (!found) throw new Error(`bytes/as-list-types: can't read ${line}`);
    const [, bytes, list, type] = found;
    return [`\`${type}\``, String(list.split(",").length), kind(type), `\`${bytes}\``, `\`${printed[i]}\``];
  });
  return { header: ["T", "Bytes each", "Reads", "From", "Gives"], rows };
}

export const bytesDoc: LessonDoc = {
  lead: "Bytes holds raw data: a file's contents, an image, a network message. It is stored compactly, one byte per byte.",
  simple: <>Underneath, every file, picture and network message is a long row of bytes. <C>Bytes</C> holds that row exactly as it is, without reading it as text, so you can store it, send it, check it for damage, or read the numbers in it.</>,
  minutes: 5,
  before: ["basics/strings"],
  body: (
    <>
      <Ex name="types/bytes" />
      <ModuleTable file="02-types/13-bytes.md" number="2.13" first="Written" />
      <H2>Checking data</H2>
      <Ex name="bytes/check" />
      <P>
        A checksum changes when the data does, even by one letter, so it shows data that was damaged on the way.{" "}
        <C>get</C> gives <C>none</C> past the end, where <C>b[i]</C> would stop the program.
      </P>
      <H2>Reading numbers</H2>
      <P>
        Data often holds numbers, such as a chunk of a file. <C>b.as_list&lt;T&gt;()</C> reads them all, without going
        through the bytes one by one. It gives <C>none</C> when the length isn&apos;t a whole number of them:
      </P>
      <Ex name="bytes/as-list" />
      <P><C>T</C> says how to read the bytes, and how many make one number:</P>
      <Table header={numberTypes().header} rows={numberTypes().rows} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The bytes are little-endian, smallest part first. Nearly all files and processors use that order, so data
          reads the same on every machine.
        </li>
        <li>The list is a copy, made in one go, with no work for each number. Changing the list leaves the bytes as they were.</li>
        <li>Any other type is refused, and so is leaving the type out:</li>
      </ul>
      <Ex name="bytes/as-list-other-type.fails" />
      <Ex name="bytes/as-list-no-type.fails" />
      <H2>One number at a place</H2>
      <P>
        A file format or a network message puts each number at its own place. <C>b.read&lt;T&gt;(at)</C> reads the
        one whose bytes start at <C>at</C>, and <C>Bytes.of&lt;T&gt;(n)</C> gives a number&apos;s bytes, to build a
        message with <C>+</C>:
      </P>
      <Ex name="bytes/read-of" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The bytes are little-endian unless <C>big_endian: true</C>, the order network protocols and some file formats use.</li>
        <li>A <C>read</C> whose bytes don&apos;t all fit gives <C>none</C>, as <C>b.get(i)</C> does past the end. So does a negative <C>at</C>.</li>
        <li><C>T</C> is a number of fixed size, as for <C>as_list</C>:</li>
      </ul>
      <Ex name="bytes/read-other-type.fails" />
      <H2>A whole list as bytes</H2>
      <P>
        <C>Bytes.of_list&lt;T&gt;(xs)</C> is the reverse of <C>as_list</C>: every number in the list as bytes, in
        order, to write to a file or send. It copies them all at once, instead of joining a <C>Bytes.of</C> for each
        number:
      </P>
      <Ex name="bytes/of-list" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each number&apos;s bytes are as <C>Bytes.of&lt;T&gt;</C> gives them, little-endian unless <C>big_endian: true</C>.</li>
        <li><C>as_list&lt;T&gt;</C> reads them back, so the round trip gives the same list.</li>
        <li><C>T</C> is a number of fixed size, and must be written:</li>
      </ul>
      <Ex name="bytes/of-list-no-type.fails" />
      <SrcRef file="tests/conformance/02-types/bytes-of-list-gives-a-lists-bytes.kav" label="Bytes.of_list, tested" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/02-types/bytes-read-numbers-at-a-place.kav" label="read, tested" />
        <SrcRef file="tests/conformance/02-types/bytes-of-gives-a-numbers-bytes.kav" label="Bytes.of, tested" />
        <SrcRef file="tests/conformance/02-types/read-needs-a-fixed-size-number.kav" label="fixed-size numbers, tested" />
      </div>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/13-bytes.md" label="spec 2.13 · bytes" />
        <SrcRef file="tests/conformance/02-types/bytes-read-as-numbers.kav" label="reading numbers, tested" />
        <SrcRef file="tests/conformance/02-types/as-list-needs-a-fixed-size-number.kav" label="only fixed-size numbers, tested" />
      </div>
      <H2>Text from part of the bytes</H2>
      <P>
        A line read from a file or a network often holds words at known places. <C>b.text(start, end)</C> gives the
        bytes from <C>start</C> up to <C>end</C> as text, copied once, with no <C>Bytes</C> in between:
      </P>
      <Ex name="bytes/text-range" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Places past the end are clamped, as a slice clamps them: <C>end: 100</C> stops at the last byte.</li>
        <li>Bytes that aren&apos;t valid UTF-8 give <C>none</C>, as <C>b.text()</C> does.</li>
        <li><C>b[start:end].text()</C> does the same, also copying the text once.</li>
      </ul>
      <SrcRef file="tests/conformance/02-types/bytes-text-of-a-range.kav" label="text of a range, tested" />
      <H2>A fixed length</H2>
      <P>
        <C>Bytes&lt;len: 32&gt;</C> is exactly 32 bytes: a hash, a key, a field of a file format.{" "}
        <C>crypto.sha256</C> gives one, so a function can say in its type that it returns a digest:
      </P>
      <Ex name="bytes/fixed" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Bytes&lt;len: n&gt;</C> may go wherever <C>Bytes</C> may.</li>
        <li>Plain <C>Bytes</C> become a <C>Bytes&lt;len: n&gt;</C> only through <C>try</C> or <C>catch</C>, which fails with a <C>SizeError</C> when the length is another.</li>
        <li><C>a + b</C> and the methods that make new bytes give plain <C>Bytes</C>.</li>
        <li>The number in the type works as for lists: see <Link href="/docs/types/generics/">Generics</Link>.</li>
      </ul>
      <P>An index written as a number is checked while compiling:</P>
      <Ex name="bytes/fixed-index.fails" />
      <P><C>Bytes([…])</C> of a list literal of <C>n</C> numbers is a <C>Bytes&lt;len: n&gt;</C>, so it doesn&apos;t fit another length:</P>
      <Ex name="bytes/fixed-literal.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0137-numbers-in-types.md" label="decision 0137" />
        <SrcRef file="tests/conformance/02-types/fixed-bytes-fit-where-bytes-are-expected.kav" label="fixed length, tested" />
        <SrcRef file="tests/conformance/02-types/fixed-bytes-index-past-the-end-is-an-error.kav" label="index, tested" />
        <SrcRef file="tests/conformance/02-types/bytes-of-a-list-literal-have-its-length.kav" label="literals, tested" />
      </div>
      <H2>In JSON</H2>
      <P>
        JSON has no bytes, so <C>Bytes</C> is written as base64 text, as Protocol Buffers&apos; JSON and OpenAPI write
        them. Decoding reads the URL-safe alphabet too, with or without padding:
      </P>
      <Ex name="json/bytes-base64" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It holds for a bare value as for a field, and in lists, dicts and optionals.</li>
        <li>Text that isn&apos;t base64 is a <C>Mismatch</C> error.</li>
        <li>A <C>spawn process</C> passes <C>Bytes</C> this way too.</li>
        <li><C>ask&lt;T&gt;</C> still refuses <C>Bytes</C>: models write base64 badly.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/12-json.md" label="spec 2.12 · Bytes in JSON" />
        <SrcRef file="tests/conformance/02-types/json-writes-bytes-as-base64.kav" label="base64, tested" />
        <SrcRef file="tests/conformance/09-concurrency/spawn-process-passes-bytes.kav" label="spawn process, tested" />
      </div>
      <Note>Files are read as <C>Bytes</C> with <C>fs.read_bytes</C>, or in pieces with <C>fs.chunks</C>: see <Link href="/docs/standard-library/files/reading/">Reading files</Link>.</Note>
    </>
  ),
};
