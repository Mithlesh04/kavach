import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P, SpecCode } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- The Machine ----------
//
// Spec 14.25. What a machine says differs on every machine, and some of
// it is private, so the samples print only what is true on all of them:
// never a hostname, a processor's name or a memory size.

const FILE = "14-standard-library/25-system.md";
const TESTS = "tests/conformance/14-standard-library";
const fakeGpus = loadProject("system/fake-gpus", ["src/main.kav"]);

export const systemDoc: LessonDoc = {
  lead: "std.system says what machine the program runs on: its system, processor, memory, GPUs, drives and devices, and the programs and users on it. The basics are free; what could tell one machine from another needs System, and what is critical needs System.critical.",
  simple: <>Before you split a job between workers, you want to know how many workers the machine has. <C>std.system</C> answers that, and more: how much memory there is, which GPUs, how long the machine has been on. The questions that could identify a machine need your permission first.</>,
  minutes: 8,
  before: ["agents/capabilities", "standard-library/running-programs"],
  body: (
    <>
      <P>The most common use: split work across every core the machine has.</P>
      <Ex name="system/sizing" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>system.cores()</C> gives <C>logical</C>, how many threads can run at once, and <C>physical</C>, the cores behind them.</li>
        <li><C>system.os()</C> gives the system&apos;s name and version, and <C>system.arch()</C> the processor&apos;s architecture, such as <C>&quot;x86_64&quot;</C>.</li>
        <li>These three need no capability, like the clock: many machines share each answer, so they identify no one.</li>
      </ul>
      <H2>The rest needs System</H2>
      <P>
        Memory, the processor&apos;s name, the GPUs, the hostname and the uptime together can tell one machine from
        another. So a function that reads them says <C>uses System</C>:
      </P>
      <Ex name="system/details" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>system.memory()</C> gives <C>total</C> and <C>available</C> in bytes: what a program could still take without the system swapping.</li>
        <li><C>system.gpus()</C> lists each GPU, discrete ones first. Each has a <C>kind</C>: <C>Discrete</C>, <C>Integrated</C>, <C>Virtual</C>, <C>Other</C>, or <C>Software</C> for one that draws on the processor. Listing them opens no GPU.</li>
        <li><C>system.uptime()</C> is a <C>Duration</C>.</li>
        <li><C>kavach audit</C> lists <C>System</C>, and an agent gets it only when a person approves.</li>
      </ul>
      <P>A <C>pub</C> function that reads them without saying so doesn&apos;t compile:</P>
      <Ex name="system/needs.fails" />
      <H2>Testing GPU code with a fake GPU list</H2>
      <P>
        Code that picks a GPU should be tested with GPUs the test machine doesn&apos;t have. The setting{" "}
        <C>KAVACH_FAKE_GPUS</C> replaces the list <C>system.gpus()</C> gives with one of its own, ranked the same way:
        discrete first, then integrated, the rest last.
      </P>
      <CodeBlock code={fakeGpus.files[0].code} file="src/main.kav" />
      <Terminal session={fakeGpus.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each entry is <C>name:vendor:kind:backend</C>, and entries are separated by <C>;</C>. A name may have spaces: quote the whole value in a shell, <C>KAVACH_FAKE_GPUS=&quot;AMD Radeon RX 7900:0x1002:discrete:vulkan&quot;</C>.</li>
        <li>The vendor is written as a PCI vendor ID, such as <C>0x10de</C>, and read back as the maker&apos;s name: NVIDIA, AMD, Intel, Apple, Qualcomm, ARM or Microsoft. An ID it doesn&apos;t know reads as <C>&quot;&quot;</C>.</li>
        <li>The Intel GPU was listed first, but the discrete AMD one comes first in the list.</li>
        <li>It changes only the list. The GPU that runs <C>map</C> is still the machine&apos;s.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.25" />
        <SrcRef file="tools/cli/tests/fake_gpus.rs" symbol="a_fake_gpu_list_is_ranked_like_the_machines" label="tested" />
        <SrcRef file="runtime/gpu/src/listing.rs" line={59} symbol="fn vendor" label="vendor names, defined here" />
      </div>
      <H2>Every part of the machine</H2>
      <P>
        Drives, volumes, network adapters, displays, the battery, sensors and every device the system knows come as
        typed records, and need <C>System</C> too. Each call reads the machine again, so a pen drive plugged in shows
        in the next call. What a system doesn&apos;t say is <C>&quot;&quot;</C>, 0 or an empty list, never an error:
      </P>
      <Ex name="system/parts" />
      <P>From the spec: free space, the battery and the Bluetooth devices. It prints this machine&apos;s details, so it is shown here, not run:</P>
      <SpecCode file={FILE} number="14.25" text={"fn report()"} />
      <ModuleTable file="25-system.md" number="14.25" first="Function" heading="Every part of the machine" />
      <H2>What is critical needs System.critical</H2>
      <P>
        Some things are critical to the machine and the people on it. <C>System.critical.read</C> reads them: the
        applications installed, the serial numbers and network addresses that name this exact machine, the programs
        running and the ports they hold. <C>System.critical.write</C> changes the machine for everyone on it.
      </P>
      <P>
        <C>uses System</C> doesn&apos;t cover either. A function writes them by name, <C>kavach audit</C> lists them
        apart, and an agent gets one only when a person approves:
      </P>
      <Ex name="system/critical.fails" />
      <H2>Programs, ports and users</H2>
      <P>
        For the engineers who look after a machine: the programs running, the network ports they hold, and its users.
        Reading the programs and ports needs <C>System.critical.read</C>; the user this program runs as needs only{" "}
        <C>System</C>:
      </P>
      <Ex name="system/programs" />
      <P>
        From the spec: free port 8080 by ending the programs that listen on it. It changes the machine, so it is shown
        here, never run:
      </P>
      <SpecCode file={FILE} number="14.25" text={"fn free_8080()"} />
      <ModuleTable file="25-system.md" number="14.25" first="Function" heading="Programs, ports and users" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          A change needs the system&apos;s own permission too. A program not run as an administrator, or as root, can end
          only its own user&apos;s programs, and can&apos;t add or remove users. A change refused fails with{" "}
          <C>SystemError.Denied</C>.
        </li>
        <li><C>SystemError</C> is <C>NotFound(what:)</C>, <C>Denied(reason:)</C> or <C>Failed(reason:)</C>.</li>
        <li>
          A port found free can be taken by another program before yours listens on it. A server that can should listen
          on port 0 and read the port it got.
        </li>
        <li>Tests can&apos;t change the machine: a test that calls one of these changes is refused.</li>
      </ul>
      <H2>A report&apos;s header</H2>
      <P>
        From the spec: the line a bug report or a benchmark starts with. It prints this machine&apos;s details, so it
        is shown here, not run:
      </P>
      <SpecCode file={FILE} number="14.25" text={"fn header()"} />
      <H2>Every function</H2>
      <ModuleTable file="25-system.md" number="14.25" first="Function" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>No read fails: what a system doesn&apos;t say comes back as <C>&quot;&quot;</C> or 0.</li>
        <li>The time your own program has spent on the processor is <C>process.cpu_ms()</C>, on <C>std.process</C>.</li>
      </ul>
      <Note>Not yet: each GPU&apos;s memory, and what changes while the program runs, such as the processor&apos;s current speed.</Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.25 · std.system" />
        <SrcRef file={`${TESTS}/system-says-what-machine-it-is.kav`} label="tested" />
        <SrcRef file={`${TESTS}/system-details-need-the-capability.kav`} label="System, tested" />
        <SrcRef file="stdlib/system/system.kav" label="system.kav" />
        <SrcRef file="docs/decisions/0110-std-system.md" label="decision 0110" />
        <SrcRef file={`${TESTS}/system-parts-come-typed.kav`} label="every part, tested" />
        <SrcRef file={`${TESTS}/system-critical-is-not-covered-by-system.kav`} label="System.critical, tested" />
        <SrcRef file={`${TESTS}/system-programs-ports-and-users.kav`} label="programs, ports and users, tested" />
        <SrcRef file={`${TESTS}/tests-cant-change-the-machine.kav`} label="tests can't change the machine, tested" />
        <SrcRef file="docs/decisions/0112-every-detail-of-the-machine.md" label="decision 0112" />
      </div>
    </>
  ),
};
