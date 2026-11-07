import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { C } from "@/components/inline-code";
import { fileTables } from "@/lib/spec-tables";

// The frames benchmark on the GPU records page (benchmarks/gpu/README.md):
// its table, and what it shows, worked out from the README's numbers.

const FRAMES = "benchmarks/gpu/README.md";
/** The frames benchmark's rows, as the README writes them. */
const frames = () => {
  const t = fileTables(FRAMES).find((x) => x.header.join("|") === "Way|300 frames");
  if (!t) throw new Error(`${FRAMES}: no Way|300 frames table`);
  return t.rows.map(([way, ms]) => ({ way: way.replace(/\*\*/g, "").replace(/`/g, ""), ms: ms.replace(/\*\*/g, "") }));
};
const msOf = (way: string) => {
  const row = frames().find((r) => r.way.startsWith(way));
  if (!row) throw new Error(`${FRAMES}: no row ${way}`);
  return Number(row.ms.replace(/,/g, "").split(/[ –]/)[0]);
};

export function FramesTable() {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Way</th>
            <th className="py-2 font-bold">300 frames, ms</th>
          </tr>
        </thead>
        <tbody>
          {frames().map(({ way, ms }) => (
            <tr key={way} className={`border-b border-line ${way.startsWith("Kavach, GPU") ? "font-semibold text-ink" : "text-muted"}`}>
              <td className="py-2 pr-4">{way}</td>
              <td className="py-2">{ms}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** A number the README's findings say, found by the words around it. */
const saidIn = (words: RegExp) => {
  const text = fs.readFileSync(path.join(process.cwd(), "..", FRAMES), "utf8");
  const m = text.match(words);
  if (!m) throw new Error(`${FRAMES}: no ${words}`);
  return m[1];
};

/** A ratio as the page says it: one decimal under 10, whole above. */
const ratio = (n: number) => (n >= 10 ? Math.round(n) : Math.round(n * 10) / 10);

/** What the frames table shows, worked out from its numbers. */
export function FramesFindings() {
  const gpu = msOf("Kavach, GPU");
  const cpu = msOf("Kavach, CPU (");
  const inPlace = msOf("Kavach, CPU, the step in");
  const one = msOf("C++ -O2");
  const opening = saidIn(/waits about ([\d,]+) ms for the GPU to open/);
  const frame = saidIn(/frames take about [\d,]+ ms, ([\d.]+) ms each/);
  const rest = saidIn(/frames take about ([\d,]+) ms, [\d.]+ ms each/);
  const kernel = saidIn(/300 frames of Kavach's own kernel take ([\d–]+) ms/);
  const vector = saidIn(/as one `vec4<f32>` takes ([\d–]+) ms/);
  const outside = saidIn(/The other (\d+) ms of the \d+ is the last line inside the timer/);
  const readBack = saidIn(/and the read ([\d–]+) ms/);
  const vec4Gain = saidIn(/frames took [\d–]+ ms against [\d–]+ ms, ([\d–]+)% less/);
  const cuda = msOf("CUDA");
  const cudaOpening = saidIn(/Opening the GPU takes CUDA ([\d,]+) ms/);
  return (
    <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
      <li>
        <strong className="text-ink">Most of Kavach&apos;s GPU time is opening the GPU.</strong> The first{" "}
        <C>map</C> waits about {opening} ms for it, once per program; most of that is the GPU&apos;s driver at work.
        After that, a frame takes the GPU {frame} ms, and one C++ core {Math.round((one / 300) * 10) / 10} ms.
      </li>
      <li>
        <strong className="text-ink">CUDA&apos;s frames take {cuda} ms, Kavach&apos;s about {rest}.</strong> Both write
        each particle back where it was, so the particles can stay in the GPU&apos;s fast memory from frame to frame.
        Kavach does it when nothing but the <C>map</C> holds the list, as in <C>ps = ps.map(step)</C>; a list another
        variable still holds is never written over. Run on its own, the GPU code Kavach makes for the step takes{" "}
        {kernel} ms for the frames, as fast as the same written by hand. Written by hand to read each particle as one
        block of four numbers, it takes {vector} ms. So Kavach now sends a record of four 32-bit fields to the GPU as one
        such block, in and out: side by side with the build before, the frames took {vec4Gain}% less time. The other{" "}
        {outside} ms is the last line inside the timer, <C>ps[999999]</C>: it brings the whole list back from the GPU,
        16 MB, into fresh memory the system first fills with zeros. Timed apart, that read takes {readBack} ms.
        Opening the GPU takes CUDA {cudaOpening} ms.
      </li>
      <li>
        A <C>map</C> whose list stays on the GPU doesn&apos;t wait for the GPU to finish, so frames run back to back. A
        panic then shows a little later: see{" "}
        <Link href="/docs/speed/gpu/same-results-as-the-cpu/">Same results as the CPU</Link>.
      </li>
      <li>
        <strong className="text-ink">
          On the CPU, <C>ps = ps.map(…)</C> is {ratio(inPlace / one)} times slower than one C++ core with the step in
          the lambda, and {ratio(cpu / one)} times through the named <C>gpu fn</C>.
        </strong>{" "}
        Either way, each frame writes each particle back where it is, as C++ does, instead of making a new list.
      </li>
      <li>On the GPU, the same frames take {ratio(cpu / gpu)} times less time than through the <C>gpu fn</C> on the CPU, and all the ways give the same last particle.</li>
      <li>Heavier work per item, more steps or forces between neighbours, is where the GPU wins: see the races on <Link href="/docs/speed/gpu/">GPU</Link>.</li>
    </ul>
  );
}
