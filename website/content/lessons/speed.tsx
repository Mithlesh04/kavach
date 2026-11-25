import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { CoresSplit } from "@/components/cores-split";
import { Explainer } from "@/components/explain/explainer";
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
  ["Values never alias", "a list has one owner, so the compiler can keep values in registers and split loops over the cores safely", "/docs/memory/values-not-references/"],
  ["Loops on all cores", "on cores before a for splits it over every core, with the one-core result exactly", "/docs/speed/loops-on-all-cores/"],
  ["Packed small numbers", "a list of bytes or Bools takes one byte per item, so eight times as many fit in the fast caches", "/docs/speed/byte-packed-lists/"],
  ["Radix sort", "whole numbers sort in a few passes by digit, not by comparing pairs", "/docs/speed/fast-sorting/"],
  ["Fused pipelines", "a chain of map, filter and sum runs as one loop, with no list built between the steps", "/docs/collections/lists/map-filter-sum/"],
  ["The GPU", "a map with a gpu fn runs on thousands of GPU threads", "/docs/speed/gpu/"],
];

export const howFastDoc: LessonDoc = {
  lead: "Kavach is fast because it does less work: no interpreter, no garbage collector, no copies it can avoid, and every core when you ask for it.",
  simple: <>A fast cook wastes no steps rather than rushing. Kavach is fast the same way: it skips work that isn&apos;t needed, and when you mark a job that can be shared, it shares it among all of the computer&apos;s cores, and the result never changes.</>,
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
        None of these change what a program prints. An <C>on cores</C> loop gives the one-core answer, the GPU gives the
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
  lead: "Every loop runs on one core, in order, unless you write on cores before it. Then its rounds run on all the program's cores, and the compiler checks that the answer is the one-core answer, exactly.",
  simple: <>Like a pile of exam papers marked by several teachers: each takes the next small stack until none are left, and the totals are added at the end. You ask for it with <C>on cores</C>, and Kavach agrees only when the answer comes out exactly the same.</>,
  minutes: 6,
  before: ["basics/for-loop", "speed/how-kavach-is-fast"],
  body: (
    <>
      <CoresSplit />
      <P>
        A plain <C>for</C> runs on one core, in order, as in C, Go, Rust and JavaScript. Write <C>on cores</C>{" "}
        before it, and its rounds are cut into many small ranges, and each core takes the next free range until
        none are left. Each round here writes its own item, <C>ys[i]</C>:
      </P>
      <Ex name="speed/sub-elementwise" />
      <P>
        In <C>for (i, x) in xs.enumerate()</C>, the index <C>i</C> names the round&apos;s own slot in the same way:
      </P>
      <Ex name="speed/cores-enumerate" />
      <P>
        A round may also add whole numbers into a <C>let</C> with <C>+=</C>. Each range is summed on its own, and the
        sums are added at the end, since whole numbers add up the same in any grouping:
      </P>
      <Ex name="speed/sub-sum" />
      <P>
        <C>total = total + n</C> is the same as <C>total += n</C>. A sum kept modulo a number written in the code
        works too, as whole numbers give the same remainder in any grouping:
      </P>
      <Ex name="speed/cores-modulo" />
      <P>
        Native code spreads that one over the cores only where the compiler proves no part is negative; elsewhere it
        runs in order. The answer is the same either way.
      </P>
      <P>
        An overflow is still caught at the plain loop&apos;s round. The program stops with the same message the
        loop gives on one core:
      </P>
      <Ex name="speed/cores-overflow.panics" />
      <H2>What on cores refuses</H2>
      <P>
        Only three kinds of change keep the one-core answer: writing the round&apos;s own slot (<C>out[i]</C>, where{" "}
        <C>i</C> is the loop&apos;s name or its <C>enumerate()</C> index), writing its own row of a grid kept in one
        list (<Link href="#a-grid-kept-in-one-list">below</Link>), and adding whole numbers into a <C>let</C>.
        Anything else is a compile error
        that says why. <C>print</C> would show the rounds in a different order on each run:
      </P>
      <Ex name="speed/cores-print.fails" />
      <P>Adding floats in another order changes their last bits, so a float <C>+=</C> is refused, with the fix:</P>
      <Ex name="speed/cores-float.fails" />
      <P>
        Any other change to something every round shares is refused too. Here the last multiple of 7 depends on which
        round runs last:
      </P>
      <Ex name="speed/cores-shared.fails" />
      <P>
        For each of these, keep the plain loop, or collect each round&apos;s value in its own slot and finish the work
        after the loop. What a round reads is never limited.
      </P>
      <H2>How many cores</H2>
      <P>
        <C>on cores</C> uses all the cores the program may use. <C>on cores(n)</C> asks for at most <C>n</C>; asking
        for more than the machine has uses all it may, so the same program runs on any machine:
      </P>
      <Ex name="speed/cores-count" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The program&apos;s cap is all the machine&apos;s cores, or fewer: <C>cores = 4</C> under <C>[run]</C> in <C>kavach.toml</C>, or <C>KAVACH_CORES=4</C> when it starts, which wins when both are set. <C>KAVACH_CORES=1</C> keeps every loop on one core.</li>
        <li>Asking for more cores than the machine has uses all it has. With <C>KAVACH_PAR_LOG</C> set, the program says so when it starts, for example <C>kavach: 11 cores asked for, and this machine has 10: using 10</C>.</li>
        <li>A container or CPU limit counts: under <C>docker --cpus=2</C>, the cap is 2.</li>
        <li><C>system.cores()</C> reports the cap, so a program that sizes its work by it follows the cap too.</li>
      </ul>
      <P>A count of 0 or less is a compile error when it is written as a number:</P>
      <Ex name="speed/cores-zero.fails" />
      <P>and a panic when it is worked out while the program runs:</P>
      <Ex name="speed/cores-zero.panics" />
      <H2>Leaving early</H2>
      <P>
        <C>return</C>, <C>break</C>, <C>continue</C>, <C>fail</C> and <C>?</C> work as on one core: the first round
        in order that leaves decides, and rounds after it change nothing. Here the first item over 998 is at 999,
        however the rounds were shared out:
      </P>
      <Ex name="speed/cores-early" />
      <H2>Nested loops</H2>
      <P>
        <C>on</C> covers the one loop after it. A loop inside it runs on one core in each round, unless it has its own{" "}
        <C>on</C>. A label comes first: <C>rows: on cores for …</C>. Here each round builds one row and writes it to its
        own slot:
      </P>
      <Ex name="speed/cores-nested" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>while</C> can&apos;t take <C>on</C>: each round depends on the one before.</li>
        <li><C>on</C>, <C>cores</C> and <C>threads</C> are ordinary names everywhere else.</li>
      </ul>
      <H2>A grid kept in one list</H2>
      <P>
        Images and matrices are often one long list, row after row. A round may write its own row of it,{" "}
        <C>grid[row * w + col]</C> inside <C>for col in 0..w</C>, where <C>row</C> is the loop&apos;s name:
      </P>
      <Ex name="speed/cores-rows" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The parts may come in any order: <C>grid[col + w * row]</C> is the same row.</li>
        <li>
          <C>w</C> must be unable to change: a number written in the code, a <C>const</C> or a parameter. With a{" "}
          <C>let</C>, rows could overlap, so it is refused:
        </li>
      </ul>
      <Ex name="speed/cores-row-width.fails" />
      <P>
        A list is written one way in a loop: its own slots, or its rows with one <C>w</C>. Written both ways, two rounds
        could reach the same item, so that is a compile error too:
      </P>
      <Ex name="speed/cores-two-ways.fails" />
      <P>
        A built program spreads the rows over the cores by how many items they hold, so even a few long rows are
        shared out.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/04-control-flow/on-cores-writes-each-rounds-own-row.kav" label="own row, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-refuses-a-list-written-two-ways.kav" label="two ways, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-refuses-a-row-whose-width-can-change.kav" label="a width that can change, tested" />
      </div>
      <H2>Rounds that wait: on threads</H2>
      <P>
        <C>on cores</C> is for rounds that compute. For rounds that mostly wait, on the network, a disk or a timer,
        write <C>on threads(n)</C>. Its rounds run at the same time, at most <C>n</C> at once, and may do I/O and print:
      </P>
      <Ex name="speed/threads-at-once" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The three waits overlap, so the loop takes about as long as the longest, not their sum.</li>
        <li>What rounds print comes in the order of their turns: the round that waits least prints first.</li>
        <li>
          The result is still the one-core result. Each round gets copies of what it reads, and its slot writes and
          whole-number sums are applied in round order, so <C>sizes</C> is <C>[0, 10, 20]</C> on every run.
        </li>
        <li>
          Each round is a task of its own, as <C>spawn thread</C> makes it. Before round <C>i</C> starts, round{" "}
          <C>i - n</C> is waited for, as <C>parallel_map</C> does, so <C>threads(1)</C> runs one round at a time.
        </li>
        <li>A loop whose body can leave early (<C>break</C>, <C>return</C>, <C>fail</C>, <C>?</C>) runs its rounds one after another, so the first round in order that leaves decides.</li>
        <li><C>threads(n)</C> may be more than the cores, as its rounds mostly wait.</li>
        <li>
          The whole program runs at most <C>[run] threads</C> rounds at once (256 by default), across all its{" "}
          <C>on threads</C> loops. At the cap, a loop waits for its own oldest round before starting another. A loop
          with no round running still starts one, so nested loops can&apos;t wait for each other forever. With{" "}
          <C>threads = 1</C>, each round ends before the next begins, whatever <C>threads(n)</C> asks.
        </li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/04-control-flow/run-threads-caps-on-threads-rounds/src/main.kav" label="[run] threads, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-threads-rounds-run-at-once.kav" label="at once, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-threads-runs-at-most-n-rounds.kav" label="at most n, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-threads-leaving-early-runs-in-order.kav" label="leaving early, tested" />
      </div>
      <H2>Before a value</H2>
      <P>
        <C>on</C> also goes before a value, to say where the work inside it runs. <C>on cores(n)</C> limits library
        work, such as a big <C>sort</C> or a matrix <C>@</C>, to at most <C>n</C> cores. <C>on cpu</C> keeps its{" "}
        <Link href="/docs/speed/gpu/">GPU maps</Link> on the CPU:
      </P>
      <Ex name="speed/on-value" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It covers the one value after it, and gives the same value as without it.</li>
        <li>A count of 0 or less is a compile error when written as a number, and a panic when worked out while running:</li>
      </ul>
      <Ex name="speed/on-value-zero.fails" />
      <Ex name="speed/on-value-zero.panics" />
      <P>
        <C>on gpu(n) x</C> runs the GPU work of <C>x</C> on device <C>n</C>, of a machine with several GPUs: see{" "}
        <Link href="/docs/speed/gpu/choosing-a-gpu/">Choosing a GPU</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/04-control-flow/on-before-a-value-gives-the-same-value.kav" label="the same value, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-before-a-value-count-of-zero-is-an-error.kav" label="a count of 0, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-before-a-value-count-under-one-panics.kav" label="a count under 1, tested" />
      </div>
      <H2>What the machine does</H2>
      <P>The loop above, step by step, as a program built with <C>kavach build</C> runs it on 4 cores:</P>
      <Explainer id="cores-loop" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="compiler/kir/src/par.rs" line={5} symbol="outlined into a chunk function" label="the compiler moves the body out" />
        <SrcRef file="runtime/native/src/rt_par.rs" line={254} symbol="fn fan_out" />
        <SrcRef file="runtime/native/src/rt_par.rs" line={156} symbol="fn run_shares" label="taking the next free range" />
      </div>
    </>
  ),
  deeper: (
    <>
      <P>
        Each range runs quietly: where the plain loop would panic, the range returns a flag instead. The runtime then
        throws the ranges&apos; work away and runs the plain loop on one core, which panics at the exact round and line
        you would have seen. Whole-number sums also record the highest and lowest each range&apos;s total reached, so a
        total that would overflow in the middle, in one-core order, is caught too.
      </P>
      <P>
        The race below is one core each: Kavach run with <C>KAVACH_CORES=1</C>, against the plain C++ loop. Kavach&apos;s
        time is still to measure, and so is the race with both on all cores, against C++ with OpenMP. Until 2026-11-25
        this chart compared Kavach on all cores with C++ on one, which says nothing about either language.
      </P>
      <RaceChart suite="loops" id="loop" />
      <VerifyBlock suite="loops" id="loop" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/04-control-flow.md" label="spec 4.9 · where a loop runs" />
        <SrcRef file="docs/decisions/0126-loops-on-one-core-unless-on-cores.md" label="decision 0126" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-gives-the-one-core-result.kav" label="the one-core result, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-keeps-a-sum-modulo-a-number.kav" label="a sum modulo a number, tested" />
        <SrcRef file="tests/conformance/04-control-flow/on-cores-writes-the-enumerate-index-slot.kav" label="the enumerate index, tested" />
        <SrcRef file="tests/conformance/04-control-flow/big-loops-give-plain-results.kav" label="on cores(2), tested" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>A plain <C>for</C> loop, with no <C>on</C> before it, sums a million numbers. Where does it run?</>,
      options: ["On one core, in order", "On every core", "On the GPU"],
      answer: 0,
      explain: "Every loop runs on one core unless on cores says otherwise.",
    },
    {
      type: "choice",
      prompt: <>A round of an <C>on cores</C> loop hits an overflow. What does the program print?</>,
      options: ["The exact panic the one-core loop gives", "A special parallel-overflow error", "Nothing: the range is skipped"],
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
