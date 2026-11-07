import Link from "next/link";
import { GpuFlow } from "@/components/gpu-flow";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { RaceChart } from "@/components/race-chart";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { FramesFindings, FramesTable } from "./gpu-frames";
import { Streams } from "./gpu-streams";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";

const late = loadProject("gpu/late", ["src/main.kav"]);

const SPEC = "docs/spec/15-gpu.md";

// What runs where today (spec 15, its open questions, and runtime/gpu).
const where: [op: string, runs: string, note: React.ReactNode][] = [
  ["list.map(f)", "the GPU, when the list is long and f does enough work", <>with <C>f</C> a <C>gpu fn</C>, or a lambda of <C>gpu fn</C> calls</>],
  ["calling a gpu fn directly", "the CPU", <>a <C>gpu fn</C> is also an ordinary function</>],
  ["sum, min, max of Int32 or UInt32", "the GPU, when the list is already in GPU memory", <>only the result comes back; a list on the CPU stays there</>],
  ["filter of a list already in GPU memory", "the GPU, when its items are one of the four number types and its lambda fits a gpu fn", <>the kept items stay there, in their order; not yet for records and tuples</>],
  ["other filters; sum, min, max of floats", "the CPU, on every core", <>see <Link href="/docs/speed/loops-on-all-cores/">Loops on All Cores</Link></>],
  ["sort", "the CPU, on every core", <>see <Link href="/docs/speed/fast-sorting/">Fast Sorting</Link></>],
];

// ---------- GPU ----------

export const gpuDoc: LessonDoc = {
  lead: "Mark a function gpu fn and map a list with it. No buffers, no indices, no launches: the compiler and the runtime do the GPU work, and the answers are the CPU's.",
  simple: <>A CPU is like a few skilled cooks; a GPU is like a huge kitchen of helpers who all chop at once. Mark a function <C>gpu fn</C>, and Kavach may hand a long list to those helpers, with the same answers.</>,
  minutes: 6,
  before: ["collections/lists/map-filter-sum", "basics/data-types/float-and-float32"],
  body: (
    <>
      <Syntax code={"gpu fn name(x: Float32, …) -> Float32 {\n    …\n}\n\nconst results = items.map(x => name(x, …))"} />
      <Ex
        name="speed/sub-gpu"
        notes={[
          { target: "gpu fn glow", chip: "GPU FN", kind: "new", what: "It may run on the GPU, so the compiler checks it uses only what a GPU can do.", why: "a mistake is a compile error that names the rule, never a crash on the GPU." },
          { target: "0..100", chip: "32 BITS", kind: "why", what: "Inside a gpu fn, a number with no other type is an Int32 or a Float32.", why: "GPUs compute 32-bit numbers fast; 64-bit ones are slow there, or missing." },
          { target: "pixels.map(p => glow(p, gain))", chip: "ON THE GPU", kind: "new", what: "A long list goes to the GPU; gain, captured, goes with it.", why: "the code is the same either way, so it also runs on the CPU, and in the debugger." },
        ]}
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each item is a parameter: there is no index to get wrong, and nothing to copy by hand.</li>
        <li>This list has three items, so it ran on the CPU: sending it to the GPU would cost more than the work. A long list goes to the GPU, with the same code.</li>
        <li>It runs on NVIDIA, AMD, Intel and Apple GPUs, through Vulkan, DirectX 12 and Metal.</li>
      </ul>
      <H2>How a GPU map works</H2>
      <P>Step through both, or press Play: one CPU core takes the items one by one; the GPU pays for a trip over and back, then does every item at once.</P>
      <GpuFlow />
      <H2>What runs where today</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Operation</th>
              <th className="py-2 pr-4 font-bold">Runs on</th>
              <th className="py-2 font-bold">Note</th>
            </tr>
          </thead>
          <tbody>
            {where.map(([op, runs, note]) => (
              <tr key={op} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{op}</C></td>
                <td className="py-2 pr-4 text-muted">{runs}</td>
                <td className="py-2 text-muted">{note}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Note>
        The GPU wins only when each item needs real work: copying a list to the GPU costs more than doubling its
        numbers. So short lists and light work stay on the CPU, which the runtime decides for you.
      </Note>
    </>
  ),
  deeper: (
    <>
      <P>
        The compiler translates each <C>gpu fn</C> into WGSL, the shading language of WebGPU. The runtime hands the GPU
        the list&apos;s own memory (a <C>[Float32]</C> keeps 4 bytes per item, as a GPU does), runs one GPU thread per
        item, and reads the results straight into the new list. It compiles each kernel once, and starts opening the GPU
        as the program starts, so that time hides behind the program&apos;s first work.
      </P>
      <P>
        Lists a <C>map</C> read or made stay in GPU memory for a while, so mapping one again copies nothing over. You
        never see it: changing such a list copies it first, as changing any list held twice does. That is why the second
        <C>map</C> below is so much faster than the first.
      </P>
      <RaceChart suite="gpu" id="first" />
      <RaceChart suite="gpu" id="second" />
      <VerifyBlock suite="gpu" id="first" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 15 · GPU" />
        <SrcRef file="docs/architecture/gpu.md" label="how it works inside" />
        <SrcRef file="docs/decisions/0044-gpu-through-wgpu.md" label="decision 0044" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>You <C>map</C> a list of 3 numbers with a <C>gpu fn</C>. Where does it run?</>,
      options: ["On the CPU: three items aren't worth sending to the GPU", "Always on the GPU", "It doesn't compile: the list is too short"],
      answer: 0,
      explain: "The runtime sends a map to the GPU only when the list is long and the work per item big enough. The code and the results are the same either way.",
    },
  ],
};

export const gpuFnDoc: LessonDoc = {
  lead: "A gpu fn uses only what a GPU can do: 32-bit numbers and Bool, arithmetic, if, loops and other gpu fns. Anything else is a compile error that names it.",
  simple: <>A GPU&apos;s many helpers work only with plain numbers, like a room of people with calculators, cut off from the outside world. A <C>gpu fn</C> may use only what they can do, and Kavach checks this before the program runs.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="gpu/rules" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Allowed</th>
              <th className="py-2 pr-4 font-bold">Not allowed</th>
              <th className="py-2 font-bold">Why</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["Float32, Int32, UInt32, Bool", "Float, Int and other 64-bit types", "GPUs compute 32-bit numbers fast: consumer NVIDIA GPUs run 64-bit floats at 1/64 of the speed, and WebGPU's shading language has none"],
              ["those types, and records and tuples of 1 to 8 of them, as parameters and result", "text, lists, maps, pointers", "each GPU thread holds a few fixed-size numbers; it can't make or grow a value in memory"],
              ["const, let, =, if/else, while, for over 0..n, return", "for over a list, match, try, recursion", "a GPU has no call stack, and nothing a thread could loop over but its own numbers"],
              ["+ - * / % and comparisons; ** on Float32", "print, debug, capabilities (files, network, AI)", "thousands of threads run at once, cut off from the outside world"],
              ["Float32(n), Int32(x); abs, min, max, clamp; math.sqrt, floor, ceil, round, pow", "a function that isn't a gpu fn, other std modules", "the compiler can only translate what it has checked follows these rules"],
            ].map(([ok, no, why]) => (
              <tr key={ok} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{ok}</C></td>
                <td className="py-2 pr-4 text-muted">{no}</td>
                <td className="py-2 text-muted">{why}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Inside a <C>gpu fn</C>, a number with no other type is 32-bit: <C>const k = 1.5</C> is a <C>Float32</C>. Called
        directly, as in <C>brighten(0.2, 0.1)</C>, it runs on the CPU like any function. Each rule is checked when compiling:
      </P>
      <Ex name="gpu/float64.fails" />
      <Ex name="gpu/print.fails" />
      <P>Calling a plain function from a <C>gpu fn</C> is refused, and the message says whether that function could be one. This one could:</P>
      <Ex name="gpu/plain-call.fails" />
      <P>This one can&apos;t, and the message says why:</P>
      <Ex name="gpu/plain-print.fails" />
      <Ex name="gpu/list.fails" />
      <Note>So a <C>gpu fn</C> can never fail on the GPU in a way it couldn&apos;t on the CPU.</Note>
      <H2>Limits and sizes</H2>
      <Limits id="gpu" />
    </>
  ),
};

export const gpuMapDoc: LessonDoc = {
  lead: "list.map(f) runs on the GPU when f is a gpu fn, the items are 32-bit numbers, and the list is long enough to be worth it.",
  simple: <>Sending work to the GPU is like posting a parcel: worth it for a big load, not for one letter. <C>map</C> sends the list to the GPU only when it is long enough and the work fits the GPU&apos;s rules.</>,
  minutes: 3,
  body: (
    <>
      <P>A million pixels, each through 100 steps. On a machine with a GPU, this <C>map</C> runs there:</P>
      <Ex name="gpu/million.native" />
      <P>
        Only a compiled program uses the GPU: <C>kavach build</C> or <C>kavach run --native</C>, as here. Plain{" "}
        <C>kavach run</C> interprets the program, so the same <C>map</C> runs on the CPU, with the same results and much
        more slowly, whatever <C>KAVACH_GPU</C> says.
      </P>
      <P>
        The lambda can capture values from outside, such as <C>gain</C>, as long as they are one of the four GPU types.
        The program checks its own results instead of printing a million numbers.
      </P>
    </>
  ),
};

export const gpuRecordsDoc: LessonDoc = {
  lead: "Items can be records or tuples of 1 to 8 GPU-type fields, and two lists can run together through collections.zip.",
  simple: <>Each item can be a small form with a few number boxes, such as a moving particle. The GPU reads the boxes and fills in a new form for each item; it never changes the old one.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="gpu/records" />
      <P>
        <C>p =&gt; step(p.x, p.v)</C> passes a record&apos;s fields to the <C>gpu fn</C>. <C>zip</C> pairs two lists into
        tuples, and <C>t.0</C> and <C>t.1</C> are each pair&apos;s items.
      </P>
      <H2>A whole record in, a whole record out</H2>
      <P>
        A <C>gpu fn</C> can also take a record and give a new one, so one <C>map</C> moves every item a step. In a
        frame loop the list stays in GPU memory from one frame to the next; only the last one comes back:
      </P>
      <Ex name="gpu/frames" />
      <P>
        Inside, it reads fields (<C>b.vy</C>, or <C>t.0</C> of a tuple) and makes a new record or tuple. It can&apos;t
        change the record it was given: the result is a new one. The answer is the same with{" "}
        <C>KAVACH_GPU=off</C>, on the CPU.
      </P>
      <H2>Light work, frame after frame</H2>
      <P>300 frames of a million light particles, against C++ and CUDA:</P>
      <FramesTable />
      <FramesFindings />
    </>
  ),
};

export const gpuSameDoc: LessonDoc = {
  lead: "The GPU gives the CPU's answers: overflow still panics with the same message, and + - * give the same bits.",
  simple: <>The GPU must give the same answers as the CPU, like two calculators that agree. A number that grows too big is still stopped with the same message; only a few kinds of maths on decimals may differ by a tiny amount, in the very last binary digit.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="gpu/overflow.panics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Whole-number arithmetic is checked on the GPU too: overflow and division by zero panic with the CPU&apos;s message, naming the same line inside the <C>gpu fn</C>. When several items fail, the GPU may report a different one first.</li>
        <li><C>+</C>, <C>-</C> and <C>*</C> on <C>Float32</C> give the same bits, unless the GPU fuses a multiply with an add (one rounding instead of two).</li>
        <li><C>/</C>, <C>%</C>, <C>**</C>, <C>math.pow</C> and <C>math.sqrt</C> may differ in the last bits, as GPUs are allowed to; <C>floor</C>, <C>ceil</C> and <C>round</C> are exact.</li>
      </ul>
      <H2>When a panic shows</H2>
      <P>
        A <C>map</C> whose list stays in GPU memory doesn&apos;t wait for the GPU to finish, so a loop of them runs
        back to back. Here the last item overflows. On the CPU the panic stops the loop; on the GPU,{" "}
        <C>looped</C> prints first:
      </P>
      <CodeBlock code={late.files[0].code} file="src/main.kav" />
      <Terminal session={late.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The panic shows the next time the program reads the list on the CPU (<C>ns[0]</C> here), when another GPU
          call waits for its own result, or when the program ends.
        </li>
        <li>Its message and line are the same either way. Only when it shows changes: code after the <C>map</C> may have run by then.</li>
        <li>To find where a panic starts, run with <C>KAVACH_GPU=off</C>.</li>
      </ul>
      <SrcRef file="docs/decisions/0061-gpu-maps-dont-wait.md" label="decision 0061 · GPU maps don't wait" />
    </>
  ),
};

export const gpuWhereDoc: LessonDoc = {
  lead: "The runtime chooses CPU or GPU for each map. KAVACH_GPU overrides it, to debug or to compare.",
  simple: <>Kavach picks the CPU or the GPU for each job, like a shop choosing a bike courier or a lorry by the size of the load. The <C>KAVACH_GPU</C> setting lets you choose yourself, to find a bug or to compare.</>,
  minutes: 2,
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">KAVACH_GPU</th>
              <th className="py-2 font-bold">Does</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["(not set)", "the runtime decides from the list's length and the work per item"],
              ["off", "everything on the CPU: use a debugger, or measure the CPU"],
              ["on", "the GPU for a list of any length"],
              ["check", "runs both, and panics at the first item whose results differ (floats compared with near)"],
            ].map(([v, d]) => (
              <tr key={v} className="border-b border-line">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{v}</C></td>
                <td className="py-2 text-muted">{d}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        The runtime estimates each <C>gpu fn</C>&apos;s work from its operations, with a loop counted once per round. A
        machine with no GPU runs everything on the CPU, with the same code and the same results.
      </P>
    </>
  ),
};

// What a gpu fn and a GPU map can and can't do (spec 15.1, 15.2), each
// row backed by a sample on this page or the ones it links.
const canUse: [what: string, ok: boolean, instead: React.ReactNode][] = [
  ["Calling another gpu fn", true, <>Split work into small <C>gpu fn</C>s that call each other.</>],
  ["if, while, for over 0..n", true, <>Loops may run as long as each item needs.</>],
  ["A pipeline: map after map", true, <>Chain them in one expression, <C>xs.map(f).map(g)</C>: they run as one pass on the GPU, and the list between them is never made.</>],
  ["sum, min or max after a GPU map", true, <>On whole numbers they run on the GPU too, and only the result comes back.</>],
  ["filter after a GPU map", true, <>It runs on the GPU too when its items are numbers, not yet records or tuples, and its lambda fits a <C>gpu fn</C>. The kept items stay there. Otherwise it runs on the CPU cores.</>],
  ["sort after a GPU map", true, <>Allowed, and it runs on the CPU cores, on the results.</>],
  ["Records and tuples as items", true, <>Of 1 to 8 fields of GPU types: see Records and two lists.</>],
  ["Pointers (Ptr)", false, <>Pass the numbers themselves; <C>map</C> hands each item over for you.</>],
  ["inout parameters", false, <>Give the new value back as the result, and <C>map</C> collects the results. This holds for a <C>gpu fn</C> called from another too: one rule for every <C>gpu fn</C>.</>],
  ["Files, network, print, AI", false, <>Read the file before the <C>map</C> and write the results after it: a GPU can&apos;t reach the outside world.</>],
  ["Text, lists or maps inside a gpu fn", false, <>The list is what <C>map</C> goes over; inside, work on one item&apos;s numbers.</>],
  ["Recursion", false, <>Use a <C>while</C> loop: a GPU has no call stack.</>],
  ["try, catch, fail, or ! in the signature", false, <>Check values with <C>if</C> and give a result for every item: on the GPU there is no error to give back.</>],
  ["Lambdas inside a gpu fn", false, <>Write another <C>gpu fn</C> and call it.</>],
];

export const gpuCanDoc: LessonDoc = {
  lead: "Pointers, inout, files and recursion stay on the CPU; helpers, loops and pipelines of maps run on the GPU. Every rule is checked when compiling.",
  simple: <>GPU code can do maths, make choices, repeat steps, and pass a list from one step to the next. It can&apos;t open files or reach the outside world. For each thing it can&apos;t do, this page shows what to write instead.</>,
  minutes: 4,
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">In GPU code</th>
              <th className="py-2 pr-4 font-bold">Allowed?</th>
              <th className="py-2 font-bold">How</th>
            </tr>
          </thead>
          <tbody>
            {canUse.map(([what, ok, instead]) => (
              <tr key={what} className="border-b border-line align-top">
                <td className="py-2 pr-4">{what}</td>
                <td className={`py-2 pr-4 font-bold ${ok ? "text-ok" : "text-bad"}`}>{ok ? "Yes" : "No"}</td>
                <td className="py-2 text-muted">{instead}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>A pipeline of maps</H2>
      <Ex name="gpu/chain" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Chained in one expression</strong>, <C>xs.map(…).map(…)</C> runs as one pass on the GPU: each item goes through both functions, and the list between them is never made, copied back or stored.</li>
        <li><strong className="text-ink">In two statements</strong>, the list in the middle, <C>lit</C>, is made, and stays only in GPU memory: only the GPU reads it, so it is never copied back.</li>
        <li>Either way the numbers are the same. On a long list the <C>filter</C> at the end runs on the GPU too, and the <C>sum</C> of floats on the CPU.</li>
      </ul>
      <H2>A loop of maps</H2>
      <P>
        A <C>map</C>&apos;s result stays on the GPU until the CPU reads it. So a loop of 100 <C>map</C>s copies the
        list back once, at the first read after the loop. A <C>sum</C>, <C>min</C> or <C>max</C> of whole numbers
        runs on the GPU too, and only its result comes back, even from <C>xs.map(f).sum(g)</C> in one expression:
      </P>
      <Ex name="gpu/loop" />
      <P>
        A loop that adds up such sums doesn&apos;t wait for each one: see{" "}
        <Link href="/docs/speed/gpu/sums-in-a-loop/">Sums in a loop</Link>.
      </P>
      <P>
        You don&apos;t see any of this: every read sees the list&apos;s items. A panic in a <C>map</C> keeps its message
        and line, but may show later: see{" "}
        <Link href="/docs/speed/gpu/same-results-as-the-cpu/">Same results as the CPU</Link>.
      </P>
      <P>
        A GPU adds the items in another order, so it checks every partial sum. If any running sum, added in order as
        the CPU would, leaves the type, the CPU adds the list again in order, so the panic names the same step:
      </P>
      <Ex name="gpu/sum-overflow.panics" />
      <Note>For speed on the GPU, keep the CPU out of the middle: chain maps in one expression, and read a GPU list only when you need its items. The less a list travels between GPU and CPU, the faster.</Note>
      <Streams />
      <H2>Helpers and loops</H2>
      <Ex name="gpu/helper" />
      <Ex name="gpu/collatz" />
      <H2>Refused when compiling</H2>
      <Ex name="gpu/pointer.fails" />
      <Ex name="gpu/inout.fails" />
      <Ex name="gpu/files.fails" />
      <Ex name="gpu/recursion.fails" />
      <Ex name="gpu/fail.fails" />
    </>
  ),
};

// ---------- GPU Use Cases ----------

const uses: [title: string, text: React.ReactNode, sample: string][] = [
  ["Images: gray from color", <>Each pixel&apos;s brightness, from its red, green and blue, with the weights screens use. The same shape works for brightness, contrast and gamma.</>, "gpu/gray"],
  ["Audio: louder without clipping", <>Every sample made louder, then <C>clamp</C>ed to -1 and 1 so it never distorts.</>, "gpu/clamp"],
  ["Maps: distance to every shop", <>The distance from home to many points, with <C>math.sqrt</C>: the first step of &quot;nearest&quot; searches.</>, "gpu/distance"],
  ["Finance: savings in ten years", <>Compound growth for many accounts at once, a loop per item.</>, "gpu/interest"],
  ["Science: the Mandelbrot set", <>Heavy work per point: up to 50 rounds each, the kind of job GPUs are built for. Each <C>#</C> is a point that never escaped.</>, "gpu/mandelbrot"],
  ["Simulation: moving particles", <>Each particle a record, stepped forward in time.</>, "gpu/records"],
];

export const gpuUsesDoc: LessonDoc = {
  lead: "Work that does the same thing to many items, with enough math in each, is GPU work: images, audio, maps, finance, science and simulation.",
  simple: <>A GPU is worth it when the same real work is done to a great many things: every pixel of a photo, every point on a map. Quick, light jobs and text are better left to the CPU.</>,
  minutes: 6,
  body: (
    <>
      {uses.map(([title, text, sample]) => (
        <section key={title} aria-label={title} className="flex flex-col gap-3">
          <H2>{title}</H2>
          <P>{text}</P>
          <Ex name={sample} />
        </section>
      ))}
      <H2>When not to use the GPU</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Light work:</strong> adding 1 to each number is faster on the CPU than the trip to the GPU.</li>
        <li><strong className="text-ink">Text, maps and files:</strong> a GPU works on numbers only.</li>
        <li><strong className="text-ink">Sorting and float totals:</strong> today these run on all CPU cores, which is already fast; see <Link href="/docs/speed/loops-on-all-cores/">Loops on All Cores</Link>. Whole-number <C>sum</C>, <C>min</C> and <C>max</C> of a list already on the GPU run there.</li>
      </ul>
      <Note>These lists are short so the output fits on a page; with the same code, long lists run on the GPU.</Note>
    </>
  ),
};
