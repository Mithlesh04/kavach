import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { fileFigure, fileTables } from "@/lib/spec-tables";
import { Ex, H2, P } from "./parts";
import { Cell, Table } from "./spec-table";

// The streams benchmark on the GPU page (benchmarks/gpu/README.md): what was
// measured and its table, as the README writes them.

const README = "benchmarks/gpu/README.md";
const measured = () => fileFigure(README, /A (\d+ GB file of `Int32`s, \d+ chunks of \d+ MB), read with `fs\.chunks`/);
const runs = () => fileFigure(README, /Each row ran (\d+ times), reading ahead off and on in turns/);
const streams = () => {
  const t = fileTables(README).find((x) => x.header[0] === "Work");
  if (!t) throw new Error(`${README}: no table of work on a stream`);
  return t;
};

const left = () => fileFigure(README, /Left of decision 0067: (.+?) \(its open question\)/);

/** What a loop over a big file's chunks does today, and what decision 0067 still has to build. */
export function Streams() {
  return (
    <>
      <H2>A file bigger than a chunk</H2>
      <P>
        A <C>for chunk in fs.chunks(path, size)</C> loop with pieces of 1 MB or more reads ahead: in a native program,
        the next piece is read on a helper thread while the loop works on this one. There is nothing to write. A{" "}
        <Cell text={measured()} />, on Windows. Each row ran {runs()}, reading ahead off and on in turns, in ms:
      </P>
      <Table header={streams().header} rows={streams().rows} wrapFirst />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The file was already in the system&apos;s cache, so the disk was as fast as it gets. A file read from the disk itself isn&apos;t measured.</li>
        <li>The gain on the 200 steps is larger than the disk&apos;s share of the time explains, and that isn&apos;t understood yet.</li>
      </ul>
      <P>
        A loop that adds up what the GPU works out doesn&apos;t wait for each sum either: see{" "}
        <Link href="/docs/speed/gpu/sums-in-a-loop/">Sums in a loop</Link>. Left of the plan for data streaming through
        the GPU: <Cell text={left()} />.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0067-streaming-data-through-the-gpu.md" label="decision 0067 · streaming through the GPU" />
        <SrcRef file="benchmarks/gpu/kavach/stream.kav" label="the 200-step benchmark" />
        <SrcRef file={README} label="the benchmarks" />
      </div>
    </>
  );
}

// ---------- Sums in a loop ----------

const sumsTable = () => {
  const t = fileTables(README).find((x) => x.header[0] === "Work on 2 GB");
  if (!t) throw new Error(`${README}: no table of sums a loop doesn't wait for`);
  return t;
};
const sumRuns = () => fileFigure(README, /(\d+ runs) in turns against a compiler built from the commit before/);
const cudaRuns = () => fileFigure(README, /CUDA's column is (\d+ runs), from a comparison in turns the same day/);
const laterRuns = () => fileFigure(README, /stopped dividing to check two numbers of 16 bits, which can't overflow: over (\d+ runs) in turns/);
const heavyBefore = () => fileFigure(README, /the 200 steps went from ([\d,–]+) to/);
const heavyAfter = () => fileFigure(README, /the 200 steps went from [\d,–]+ to ([\d,–]+), and/);
const lightLater = () => fileFigure(README, /and the 1 step stayed at ([\d,–]+ against [\d,–]+)\./);
const againTable = () => {
  const t = fileTables(README).find((x) => x.header[0] === "Steps, on 2 GB");
  if (!t) throw new Error(`${README}: no table of the second measurement`);
  return t;
};
const againRuns = () => fileFigure(README, /Measured again [\d-]+, (\d+ runs) in turns against the binaries of the day before/);
const signedCost = () => fileFigure(README, /a signed `%` took (about a [a-z]+ longer) than an unsigned one/);
const aheadLight = () => fileFigure(README, /CUDA is still ((?:about )?[\d.]+×) ahead on 1 step/);
const aheadHeavy = () => fileFigure(README, /CUDA is still (?:about )?[\d.]+× ahead on 1 step and ([\d.]+×) on 200/);
const flatUpTo = () => fileFigure(README, /Time hardly changes with the steps up to (about \d+) for either/);
const uploads = () =>
  fileFigure(README, /32 chunks held in memory went from ([\d,]+ to [\d,]+ ms \(the uploads alone [\d,]+ → [\d,]+\)), and the 1-step stream/);
const uploadStream = () => fileFigure(README, /and the 1-step stream, with the file in the system's cache, from (about [\d,]+ to [\d,]+)\./);
const uploadRuns = () => fileFigure(README, /Run (\d+ times) in turns from one build, switching between the two ways/);
const counterOff = () =>
  fileFigure(README, /over (\d+ runs in turns the same day, 200 steps went from [\d,]+ to [\d,]+ and 800 steps from [\d,]+ to [\d,]+, against CUDA's [\d,]+ and [\d,]+), with the same sums/);
const byHand = () =>
  fileFigure(README, /at 800 steps it took ([\d.]+ s with today's checks, [\d.]+ s without them, and [\d.]+ s with `wgpu`'s own counter in each loop off too, against CUDA's [\d.]+ s)\./).replace(/`([^`]+)`/g, "$1");
const hugeSize = () => fileFigure(README, /A file of (\d+ GB), larger than the GPU/);
const hugeTime = () => fileFigure(README, /A file of \d+ GB, larger than the GPU's \d+ GB and too large to stay in the system's cache, took ([\d–]+ s) in all four cases/);

const notesSample = loadProject("gpu/loop-notes", ["src/main.kav"]);

/** What to do about each reason a loop waits. The reasons themselves are read from what `kavach check` printed. */
const remedy: Record<string, string> = {
  "the loop also reads what it adds up to": "Read the total after the loop.",
  "the statement does more than work out numbers": "Keep only sums and conversions in the statement. Call the function on the total, after the loop.",
  "the loop can leave in the middle of an expression": "Write the `fail` as a statement of its own.",
  "the statement reads a value the loop changes": "Keep that value out of the statement, or set it before the loop.",
  "the round uses the sum at once": "Add it to a total, and use the total after the loop.",
  "the CPU reads the result each round": "Read the list after the loop, or use only its sum.",
};

/** Each note `kavach check` printed for the sample, beside the line of the sample it points at. */
const noteRows = () => {
  const lines = notesSample.files[0].code.split("\n");
  const found = [...(notesSample.output ?? "").matchAll(/src\/main\.kav:(\d+): note: this loop waits for the GPU each round: (.+)/g)];
  const said = new Set(found.map((m) => m[2]));
  const missing = Object.keys(remedy).filter((why) => !said.has(why));
  if (missing.length) throw new Error(`gpu/loop-notes: no note says: ${missing.join("; ")}`);
  return found.map((m) => {
    if (!remedy[m[2]]) throw new Error(`gpu/loop-notes: a new reason: ${m[2]}`);
    return [m[2], `\`${lines[Number(m[1]) - 1].trim()}\``, remedy[m[2]]];
  });
};

export const gpuSumsDoc: LessonDoc = {
  lead: "A loop that adds up what the GPU works out doesn't wait for each sum, so the next chunk is read and copied over while the GPU works.",
  simple: <>A baker puts the next tray in the oven while the last one bakes, and writes each tray&apos;s count in the book when it comes out, in order. The book ends up the same, and the baking takes less time.</>,
  minutes: 4,
  before: ["speed/gpu/map-on-the-gpu"],
  body: (
    <>
      <P>There is nothing to write. In a compiled program, a loop of this shape goes on while the GPU works:</P>
      <Ex name="gpu/sums-in-a-loop.native" />
      <P>The loop goes on when all of this holds:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A statement like <C>total += xs.map(f).sum(g)</C> sets the total, with <C>sum</C>, <C>min</C> or <C>max</C> of a list a GPU <C>map</C> or <C>filter</C> made.</li>
        <li>That statement is the only place the loop mentions <C>total</C>.</li>
        <li>It only works out numbers, from values the loop doesn&apos;t change.</li>
      </ul>
      <P>A <C>const s = xs.map(f).sum(g)</C> used once, in the next statement, counts as part of it.</P>
      <H2>Only the time changes</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each statement still runs once for each round, in order, with its checks, before any code after the loop. So <C>total</C> is right whenever it can be read.</li>
        <li>A few rounds&apos; work may be under way at once, fewer for big chunks.</li>
        <li>
          Only the moment of a panic shows. If a statement overflows, the code after it in the loop, and later rounds,
          may have run by then, as for a <C>map</C> that doesn&apos;t wait.
        </li>
        <li>Plain <C>kavach run</C> doesn&apos;t use the GPU, so its loops run as written, with the same answers.</li>
      </ul>
      <H2>Which loops wait</H2>
      <P>
        <C>kavach check</C> says which loops still wait for the GPU, and why, with the line to look at. A note isn&apos;t
        an error: it goes to standard error, and the exit code stays 0. <C>kavach check --format json</C> leaves the
        notes out. These are all the reasons:
      </P>
      <Table header={["The note says", "The line it points at", "What to do"]} rows={noteRows()} />
      <details>
        <summary className="cursor-pointer text-sm font-bold text-accent-text">The whole program, and what the check printed</summary>
        <div className="mt-3 flex flex-col gap-3">
          <CodeBlock code={notesSample.files[0].code} file="src/main.kav" />
          <Terminal session={notesSample.output ?? ""} />
        </div>
      </details>
      <H2>What it saves</H2>
      <P>
        This loop on a file of 2 GB, on the same machine as CUDA, a C++ program that manages its own overlap. {sumRuns()} in
        turns against a compiler from before this change, in ms, with the same sum every time and the same as CUDA&apos;s:
      </P>
      <Table header={sumsTable().header} rows={sumsTable().rows} wrapFirst />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          This table comes from before a cheaper check in the kernel: a multiply no longer divides to check two numbers of
          16 bits, which can&apos;t overflow. Over {laterRuns()} in turns, the 200 steps then went from {heavyBefore()} to{" "}
          {heavyAfter()} ms, and the 1 step stayed at {lightLater()}. Results and panics are the same.
        </li>
        <li>The laptop was in use while it measured, so two runs far from the rest are left out, both of the old compiler. CUDA&apos;s column is {cudaRuns()}, from a comparison in turns the same day.</li>
        <li>A file of {hugeSize()}, too large for the GPU and the system&apos;s cache, took {hugeTime()} in all four cases, Kavach and CUDA: there the disk sets the speed, and neither loop holds it up.</li>
      </ul>
      <P>
        Measured again the next day, {againRuns()} in turns against the compiler of the day before, the middle run in ms,
        with the file in the system&apos;s cache, so the disk no longer holds either side back:
      </P>
      <Table header={againTable().header} rows={againTable().rows} wrapFirst />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <C>as_list</C> no longer copies a big chunk: the chunk is read straight into the list&apos;s memory, and{" "}
          <C>as_list</C> takes it when nothing else holds the chunk.
        </li>
        <li>
          A kernel divides through unsigned numbers. On this GPU a signed <C>%</C> took {signedCost()} than an unsigned
          one. Results and panics are the same.
        </li>
        <li>
          CUDA is still {aheadLight()} ahead on 1 step and {aheadHeavy()} on 200. Up to {flatUpTo()} steps the time hardly
          changes, for either: the kernel runs while the next chunk is read, so the difference is the CPU&apos;s work for
          each chunk.
        </li>
        <li>
          Copying a chunk over to the GPU takes as long as CUDA&apos;s copy. But the GPU library made a new 64 MB buffer to
          copy through every few chunks, so Kavach now copies through two buffers of its own, used in turn. Run{" "}
          {uploadRuns()} in turns, 32 chunks held in memory went from {uploads()}, and the 1-step stream from {uploadStream()} ms. Most
          of what is left is opening the GPU, which Kavach&apos;s time includes and CUDA&apos;s doesn&apos;t.
        </li>
        <li>
          Past that, the kernel sets the speed. The same kernel written by hand, at 800 steps, took {byHand()}. That counter
          stops a kernel that never ends. So since{" "}
          <SrcRef file="docs/decisions/0077-kernels-whose-loops-end.md" label="decision 0077" />, a kernel whose every
          loop is a <C>for</C> over a range that stops before its end goes without it: over {counterOff()} ms, with the
          same sums. A <C>while</C> or a <C>..=</C> range keeps the counter.
        </li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/15-gpu.md" label="spec 15.2 · the rule" />
        <SrcRef file="docs/decisions/0067-streaming-data-through-the-gpu.md" label="decision 0067" />
        <SrcRef file="tests/conformance/15-gpu/gpu-sums-in-a-loop-run-in-order.kav" label="sums in order, tested" />
        <SrcRef file="tools/cli/tests/gpu_streams.rs" label="same results and the notes, tested" />
        <SrcRef file={README} label="the benchmark" />
        <SrcRef file="benchmarks/gpu/cpp/stream.cu" label="the CUDA program" />
      </div>
    </>
  ),
};
