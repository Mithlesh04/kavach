import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Files, H2, Note, P } from "./parts";

// ---------- Choosing a GPU ----------
//
// Spec 15.6 (decision 0139): several GPUs of any vendor, numbered from 0,
// the best first. By default the system decides; code chooses with a
// block, an argument, `on gpu(n)` or `gpu(n) fn`; KAVACH_GPU and
// [run] gpu choose with no code change. The samples that run use a
// made-up GPU list (KAVACH_FAKE_GPUS), so no page prints this machine's GPUs.

const SPEC = "docs/spec/15-gpu.md";
const TESTS = "tests/conformance/15-gpu";

export const gpuChooseDoc: LessonDoc = {
  lead: "A machine may have several GPUs, of any vendors. Kavach numbers them from 0, the best first, sends each map to the least busy one, and splits a big one across them; gpu.on, on:, on gpu(n) and gpu(n) fn choose one yourself.",
  simple: <>A laptop often has two GPUs: a fast one on its own card and a slower one inside the processor. Kavach uses the fast one first and spreads work to the other when the first is busy. You can also point a job at one GPU, as you would send a parcel with one courier.</>,
  minutes: 6,
  before: ["speed/gpu/map-on-the-gpu", "speed/gpu/cpu-or-gpu"],
  body: (
    <>
      <H2>By default the system decides</H2>
      <P>A program needs no code to use the right GPU:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <strong>The devices are ranked once</strong>, at the first GPU work: GPUs on a card of their own, then the
          ones inside the processor, then the rest, each kind in the order the system lists them. The CPU comes last. A
          software GPU, which draws on the processor, runs work only when it is chosen.
        </li>
        <li><strong>New work goes to the least busy device</strong>; on a tie, the best ranked. A list already in one GPU&apos;s memory is worked on there.</li>
        <li><strong>A device that fails is skipped</strong> from then on: one that can&apos;t be opened, is lost, or runs out of memory. Its work goes to the next device, and to the CPU after the last.</li>
      </ul>
      <P>The results are the same on any device, so only the time shows where a <C>map</C> ran.</P>
      <H2>Choosing in the code</H2>
      <P>
        <C>gpu.devices()</C> lists the devices, the best first, each with its number. Four ways choose one. Here a
        made-up GPU list stands in for the machine&apos;s (<C>KAVACH_FAKE_GPUS</C>, below), so the output is the same
        on every machine:
      </P>
      <Files name="gpu/choose" first={["src/main.kav"]} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong>A block:</strong> <C>gpu.on(device) {"{ … }"}</C> runs every GPU <C>map</C>, <C>filter</C>, <C>sum</C>, <C>min</C> and <C>max</C> in it on <C>device</C>, and the GPU work of the tasks it spawns.</li>
        <li><strong>An argument:</strong> <C>xs.map(f, on: device)</C> runs that one <C>map</C> there. It wins over a block around it.</li>
        <li><strong>A value:</strong> <C>on gpu(n) x</C> runs the GPU work of <C>x</C> on device <C>n</C>, as <Link href="/docs/speed/loops-on-all-cores/#before-a-value">on cores</Link> works before a value.</li>
        <li><strong>A function:</strong> <C>gpu(n) fn</C> runs the maps of that function on device <C>n</C>.</li>
      </ul>
      <P>
        Where several choices cover one call, the nearest wins: the call&apos;s <C>on:</C>, then the innermost{" "}
        <C>gpu.on</C>, <C>on gpu(n)</C> or <C>on cpu</C> around it, then <C>gpu(n) fn</C>, then the settings, then the
        system. A choice says only where: whether a <C>map</C> is big enough for a GPU at all is decided as before.
      </P>
      <H2>Choosing needs uses Gpu</H2>
      <P>
        Listing or choosing a device needs the <C>Gpu</C> capability, so a reader of a function&apos;s first line sees
        that it picks hardware. A <C>gpu fn</C> and its <C>map</C> with no choice need nothing:
      </P>
      <Ex name="gpu/choose-needs-gpu.fails" />
      <H2>What is refused</H2>
      <P><C>on:</C> takes a device from <C>gpu.devices()</C>, not a number:</P>
      <Ex name="gpu/on-number.fails" />
      <P><C>gpu(n) fn</C> takes the number itself, written in the code, and numbers start at 0:</P>
      <Ex name="gpu/fn-number-const.fails" />
      <Ex name="gpu/negative-device.fails" />
      <P>One <C>map</C> runs on one GPU, so a lambda can&apos;t call <C>gpu(n) fn</C>s of two numbers:</P>
      <Ex name="gpu/two-gpus.fails" />
      <H2>A device that isn&apos;t there</H2>
      <P>
        A number written in the code that names no device stops the program at its start, before any of its code runs,
        with exit code 1 and a message naming the devices the machine has. A number worked out while running is a
        panic instead:
      </P>
      <Files name="gpu/missing-device" first={["src/main.kav"]} />
      <H2>One big map across several GPUs</H2>
      <P>
        A big <C>map</C> is cut into parts, one for each GPU, as an <C>on cores</C> loop is cut across cores. It
        happens on its own, when all of these hold:
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The code chose no device: no <C>on:</C>, <C>gpu.on</C>, <C>on gpu(n)</C> or <C>gpu(n) fn</C>, and none named by <C>KAVACH_GPU</C> or <C>[run] gpu</C>. A choice sends the work there whole.</li>
        <li>The list is in main memory. A list already in one GPU&apos;s memory is worked on there.</li>
        <li>Two or more GPUs can take the work.</li>
        <li>The work is big enough to pay for moving the data: about a second of the CPU&apos;s time, with enough work for each item moved. Once each GPU has done some work, its measured speed decides whether a split ends sooner.</li>
      </ul>
      <P>
        The best GPU takes the first items, the next GPU the next ones, each part as long as that GPU&apos;s speed. The
        parts run at once, and their results come back as one list, in order. They are exactly the results one GPU, or
        the CPU, gives: each item is worked out by the same <C>gpu fn</C>, on its own. A GPU that fails during its part
        hands it to the next one, or to the CPU.
      </P>
      <P>
        <C>KAVACH_PAR_LOG=1</C> notes each split, with each GPU&apos;s items:{" "}
        <C>kavach: a map of 4096 items split across GPUs: 0 (2048), 1 (2048)</C>. For tests and comparisons,{" "}
        <C>KAVACH_GPU_SPLIT=on</C> splits every <C>map</C> that may split, at any size, and <C>off</C> never splits.
      </P>
      <SrcRef file="docs/decisions/0144-one-map-across-gpus.md" label="decision 0144 · one map across GPUs" />
      <H2>With no code change</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <C>KAVACH_GPU</C> at launch takes a maker&apos;s name (<C>nvidia</C>, <C>amd</C>, <C>intel</C>, <C>apple</C>,{" "}
          <C>qualcomm</C>, <C>arm</C> or <C>microsoft</C>: the best device of that maker), a device&apos;s number (
          <C>KAVACH_GPU=1</C>), or <C>cpu</C>. The code&apos;s own choices still win, except <C>cpu</C>, which keeps
          every GPU <C>map</C> on the CPU.
        </li>
        <li><C>[run] gpu</C> in <C>kavach.toml</C> takes the same for a project, or <C>&quot;auto&quot;</C>, the default. <C>KAVACH_GPU</C> at launch wins. See <Link href="/docs/speed/gpu/cpu-or-gpu/">CPU or GPU</Link>.</li>
        <li>Drawing uses the program&apos;s GPU: the one these name, or else the best one.</li>
        <li><C>KAVACH_PAR_LOG=1</C> notes each device when it opens, and each device skipped, with why.</li>
      </ul>
      <Note>
        To test code that chooses, <C>KAVACH_FAKE_GPUS</C> replaces the machine&apos;s list with one of its own. Work
        sent to a made-up device the machine has no GPU of that name for is skipped as a device that fails, so it runs
        on the next device, or the CPU. See <Link href="/docs/standard-library/the-machine/">The Machine</Link>.
        GPUs of one kind keep the system&apos;s order: the GPU interface Kavach uses doesn&apos;t report a device&apos;s
        memory, so they aren&apos;t ranked by it. Not built yet: a matrix product split across GPUs.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 15.6 · choosing a GPU" />
        <SrcRef file="docs/decisions/0139-choosing-gpus.md" label="decision 0139" />
        <SrcRef file="stdlib/gpu/gpu.kav" line={284} symbol="fn devices" />
        <SrcRef file="stdlib/gpu/gpu.kav" line={274} symbol="type GpuDevice" />
        <SrcRef file="compiler/checker/src/gpu_choice.rs" label="the checker's GPU choices" />
        <SrcRef file={`${TESTS}/gpu-on-gives-the-same-values.kav`} label="the same values, tested" />
        <SrcRef file={`${TESTS}/gpu-map-needs-no-capability.kav`} label="no choice, no capability, tested" />
        <SrcRef file={`${TESTS}/gpu-choosing-needs-uses-gpu.kav`} label="uses Gpu, tested" />
        <SrcRef file={`${TESTS}/gpu-missing-device-stops-at-start.kav`} label="a missing device, tested" />
        <SrcRef file={`${TESTS}/gpu-choosing-a-missing-device-panics.kav`} label="worked out while running, tested" />
        <SrcRef file={`${TESTS}/gpu-one-map-runs-on-one-gpu.kav`} label="one map, one GPU, tested" />
      </div>
    </>
  ),
};
