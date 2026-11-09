import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { CoresSplit } from "@/components/cores-split";
import { C } from "@/components/inline-code";
import { RaceChart } from "@/components/race-chart";
import { RadixDemo } from "@/components/radix-demo";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P } from "./parts";

// ---------- How Kavach Is Fast ----------

// Each way, the page that shows it, and what it saves (the pages hold the
// measured numbers, read from the benchmark READMEs).
const ways: [way: string, saves: string, page: string][] = [
  ["Native machine code", "programs compile to machine code through LLVM or Cranelift, with no interpreter and no virtual machine in between", "/docs/speed/native-executables/"],
  ["No garbage collector", "memory is freed the moment its last use ends, so a program never stops for a collector", "/docs/memory/automatic-memory/"],
  ["Values never alias", "a list has one owner, so the compiler can keep values in registers and run loops on every core safely", "/docs/memory/values-not-references/"],
  ["Loops on all cores", "a plain loop is split over every core, with the one-core result exactly", "/docs/speed/loops-on-all-cores/"],
  ["Packed small numbers", "a list of bytes or Bools takes one byte per item, so eight times as many fit in the fast caches", "/docs/speed/byte-packed-lists/"],
  ["Radix sort", "whole numbers sort in a few passes by digit, not by comparing pairs", "/docs/speed/fast-sorting/"],
  ["Fused pipelines", "a chain of map, filter and sum runs as one loop, with no list built between the steps", "/docs/collections/lists/map-filter-sum/"],
  ["The GPU", "a map with a gpu fn runs on thousands of GPU threads", "/docs/speed/gpu/"],
];

export const howFastDoc: LessonDoc = {
  lead: "Kavach is fast because it does less work: no interpreter, no garbage collector, no copies it can avoid, and every core when a loop allows it.",
  simple: <>A fast cook wastes no steps rather than rushing. Kavach is fast the same way: it skips work that isn&apos;t needed, and when a job can be shared, it shares it among all of the computer&apos;s cores, and the result never changes.</>,
  minutes: 4,
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">How</th>
              <th className="py-2 font-bold">What it saves</th>
            </tr>
          </thead>
          <tbody>
            {ways.map(([way, saves, page]) => (
              <tr key={way} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><Link href={page}>{way}</Link></td>
                <td className="py-2 text-muted">{saves}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        None of these change what a program prints. A loop on all cores gives the one-core answer, the GPU gives the
        CPU&apos;s, and a sum of floats keeps its order: speed never changes a result.
      </P>
      <Note>
        Every speed claim on this site comes from a benchmark you can run yourself, against C++ and Python, with the
        exact commands: see <Link href="/docs/benchmarks/">Benchmarks</Link>.
      </Note>
    </>
  ),
};

// ---------- Loops on All Cores ----------

export const coresDoc: LessonDoc = {
  lead: "You write the plain loop. The compiler proves it can be split, the runtime runs it on every core, and the answer is the one-core answer, exactly.",
  simple: <>Like splitting a pile of exam papers among several teachers: each marks a part, and the totals are added at the end. Kavach does this for your loop on its own, and only when the answer comes out exactly the same.</>,
  minutes: 5,
  before: ["basics/for-loop", "speed/how-kavach-is-fast"],
  body: (
    <>
      <CoresSplit />
      <H2>Loops that spread out</H2>
      <P>Ordinary loops, nothing parallel written anywhere. Element-wise work, where each round writes its own item:</P>
      <Ex name="speed/sub-elementwise" />
      <P>A whole-number sum: each core sums its part, and the parts are added at the end, since whole numbers add up the same in any grouping. Overflow is still caught at the plain loop&apos;s round:</P>
      <Ex name="speed/sub-sum" />
      <P>Even a running total kept under a modulus: the compiler proves it equals the flat sum taken modulo once.</P>
      <Ex name="speed/sub-chain" />
      <H2>Loops that stay on one core</H2>
      <Ex name="speed/sub-serial" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A sum of floats keeps its order: adding them in another order would change the last digits, so it isn&apos;t split.</li>
        <li>Rounds that need the round before can&apos;t be split, and neither can a loop that prints.</li>
        <li>Small loops stay on one core too: splitting them would cost more than it saves.</li>
      </ul>
    </>
  ),
  deeper: (
    <>
      <P>
        Each chunk runs quietly: where the plain loop would panic, the chunk returns a flag instead. The runtime then
        throws the chunks&apos; work away and runs the plain loop on one core, which panics at the exact round and line
        you would have seen. Whole-number sums also record the highest and lowest each chunk&apos;s total reached, so a
        total that would overflow in the middle, in one-core order, is caught too.
      </P>
      <RaceChart suite="loops" id="loop" />
      <VerifyBlock suite="loops" id="loop" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0031-loops-on-all-cores.md" label="decision 0031" />
        <SrcRef file="tests/conformance/04-control-flow/big-sums-keep-their-panic-round.kav" label="the overflow test" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>A chunk running on another core hits an overflow. What does the program print?</>,
      options: ["The exact panic the one-core loop gives", "A special parallel-overflow error", "Nothing: the chunk is skipped"],
      answer: 0,
      explain: "The loop runs again on one core and panics at the same round, with the same message.",
    },
  ],
};

// ---------- Byte-packed Lists ----------

export const packedDoc: LessonDoc = {
  lead: "A list of bytes stores bytes: [UInt8], [Int8] and [Bool] take one byte per item, so binary work runs at cache speed.",
  simple: <>Like packing small things in a small box instead of a big one: Kavach stores each small number in one byte, so many more fit in the processor&apos;s fast memory, and work on them goes faster.</>,
  minutes: 3,
  before: ["collections/lists", "basics/data-types/sized-integers"],
  body: (
    <>
      <Ex name="speed/sub-packed" />
      <P>
        Nothing in the code changes: <C>len</C>, printing and indexing work as for any list. Many languages store each
        item as a full 8-byte word or a pointer; Kavach fits eight items in that space, so eight times as many fit in each
        line of the processor&apos;s cache.
      </P>
      <H2>Bytes and checksums</H2>
      <Ex name="speed/sub-bytes" />
      <P><C>Bytes(list)</C> copies whole words at a time, and <C>crc32c</C> runs on the processor&apos;s own CRC instructions.</P>
      <H2>Still checked</H2>
      <Ex name="speed/sub-overflow.panics" />
    </>
  ),
  deeper: (
    <>
      <RaceChart suite="bytes" id="crc" />
      <VerifyBlock suite="bytes" id="crc" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0030-byte-packed-lists.md" label="decision 0030" />
        <SrcRef file="docs/decisions/0045-packed-lists-of-small-numbers.md" label="decision 0045" />
      </div>
    </>
  ),
};

// ---------- Fast Sorting ----------

export const radixDoc: LessonDoc = {
  lead: "Long lists of whole numbers sort with a radix sort, a few passes by digit instead of comparing pairs, with buckets on every core.",
  simple: <>A post office doesn&apos;t sort letters by comparing them two at a time: it drops each into a box by its postcode, one digit after another. A radix sort sorts numbers the same way, which saves work on long lists.</>,
  minutes: 3,
  before: ["collections/sorting"],
  body: (
    <>
      <RadixDemo />
      <Ex name="speed/sub-numbers" />
      <P>
        The same <C>sort()</C> you always write: the algorithm is chosen for you. Text and floats need comparing, so
        they use a stable merge sort. Which algorithm runs for which list, and why, is on{" "}
        <Link href="/docs/collections/sorting/">Sorting</Link>.
      </P>
    </>
  ),
  deeper: (
    <>
      <RaceChart suite="loops" id="sort" />
      <VerifyBlock suite="loops" id="sort" />
      <div><SrcRef file="runtime/native/src/radix.rs" label="the radix sort" /></div>
    </>
  ),
};

// ---------- As Data Grows ----------

const tiers = [
  ["thousands", "Thousands"],
  ["millions", "Millions"],
  ["tens-of-millions", "Tens of millions"],
  ["hundreds-of-millions", "Hundreds of millions"],
];

export const sizesDoc: LessonDoc = {
  lead: "Speed at one size can mislead. The same reads and sorts at four sizes, from data that fits in the fastest cache to data that only fits in memory.",
  simple: <>A book on your desk is quick to reach; one in the basement is slow. A small list sits in the processor&apos;s fast memory, a huge one only in slower main memory, so this page measures speed at several sizes.</>,
  minutes: 4,
  before: ["speed/how-kavach-is-fast"],
  body: (
    <>
      <P>
        A thousand numbers fit in the processor&apos;s smallest, fastest cache. A hundred million fit in no cache, so
        every read that jumps somewhere new waits for main memory. The same programs run at each size, in Kavach, C++
        and Python.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">index</strong>: 50 million reads at random places in a list. The work is the same at every size; only the list&apos;s size changes.</li>
        <li><strong className="text-ink">sort</strong>: sorting the whole list, more work as it grows.</li>
        <li>Billions aren&apos;t measured: a billion whole numbers is 8 GB per language, more than the test machine holds with two running side by side.</li>
      </ul>
      {(["index", "sort"] as const).map((bench) => (
        <section key={bench} aria-label={bench} className="flex flex-col gap-3">
          <H2>{bench === "index" ? "index: 50 million reads from a list of N items" : "sort: N whole numbers"}</H2>
          {tiers.map(([id, title]) => (
            <div key={id} className="flex flex-col gap-1.5">
              <p className="m-0 text-[13px] font-bold text-soft">{title}</p>
              <RaceChart suite="sizes" id={`${bench}-${id}`} />
            </div>
          ))}
        </section>
      ))}
      <P>Each program takes its size as <C>--items N</C>, so one command reproduces every size:</P>
      <VerifyBlock suite="sizes" id="index-millions" />
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>Why do reads get slower as the list grows, though the number of reads is the same?</>,
      options: ["The list stops fitting in the caches, so reads wait for main memory", "Bigger lists have slower indexes", "The language checks more bounds"],
      answer: 0,
      explain: "The work is identical; only where the data lives changes.",
    },
  ],
};

// ---------- Native Executables ----------

const native = loadProject("speed/native", ["hello.kav"]);

export const nativeDoc: LessonDoc = {
  lead: "kavach build turns a program into one executable file of native machine code, about half a megabyte, that runs on its own.",
  simple: <>Your program becomes one file the computer understands directly, like a translated book that needs no translator at hand. Kavach&apos;s own runtime is packed inside it, so there is nothing of Kavach to install.</>,
  minutes: 3,
  before: ["tools/the-kavach-command"],
  body: (
    <>
      <CodeBlock code={native.files[0].code} file="hello.kav" />
      <Terminal commands session={"kavach build hello.kav\n./hello"} />
      <P>The executable prints what the program prints. To compile to machine code and run at once, without writing a file:</P>
      <Terminal session={native.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The runtime, memory, tasks and printing, is linked into the file: there is no virtual machine to install.</li>
        <li><C>kavach run --native</C> compiles to machine code in memory and runs it at once, without writing a file.</li>
        <li>Building needs the system&apos;s linker: the C++ Build Tools on Windows, a C compiler on Linux and macOS.</li>
        <li>Windows and Linux are tested on every change; macOS is written but not yet tested.</li>
      </ul>
      <Note>
        On Windows, an executable needs nothing to be installed: only the Universal C runtime, which every Windows
        from 10 on has. It doesn&apos;t need the Visual C++ runtime (<C>VCRUNTIME140.dll</C>).
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0016-native-runtime-and-executables.md" label="decision 0016" />
        <SrcRef file="docs/decisions/0018-llvm-backend-through-llvm-tools.md" label="decision 0018 · LLVM" />
      </div>
    </>
  ),
};
