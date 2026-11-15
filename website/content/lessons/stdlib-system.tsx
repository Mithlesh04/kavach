import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith, specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable, Table } from "./spec-table";

// ---------- The Machine ----------
//
// Spec 14.25. What a machine says differs on every machine, and some of
// it is private, so the samples print only what is true on all of them:
// never a hostname, a processor's name or a memory size.

const FILE = "14-standard-library/25-system.md";
const TESTS = "tests/conformance/14-standard-library";

/**
 * The spec's table of every part of the machine, without the two rows
 * whose capability is being renamed (apps and ids): they come back when
 * the new name lands.
 */
const parts = () => {
  const t = specTables("14-standard-library/25-system.md", "14.25").find((x) => x.heading === "Every part of the machine");
  if (!t) throw new Error("spec 14.25: no table under Every part of the machine");
  return { header: t.header, rows: t.rows.filter((r) => !/system\.(apps|ids)\(/.test(r[0])) };
};

export const systemDoc: LessonDoc = {
  lead: "std.system says what machine the program runs on: its system, processor, memory and GPUs. The basics are free; what could tell one machine from another needs the capability System.",
  simple: <>Before you split a job between workers, you want to know how many workers the machine has. <C>std.system</C> answers that, and more: how much memory there is, which GPUs, how long the machine has been on. The questions that could identify a machine need your permission first.</>,
  minutes: 6,
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
      <H2>Every part of the machine</H2>
      <P>
        Drives, volumes, network adapters, displays, the battery, sensors and every device the system knows come as
        typed records, and need <C>System</C> too. Each call reads the machine again, so a pen drive plugged in shows
        in the next call. What a system doesn&apos;t say is <C>&quot;&quot;</C>, 0 or an empty list, never an error:
      </P>
      <Ex name="system/parts" />
      <P>From the spec: free space, the battery and the Bluetooth devices. It prints this machine&apos;s details, so it is shown here, not run:</P>
      <CodeBlock code={specBlockWith(FILE, "14.25", "fn report()")} file="from spec 14.25" />
      <Table header={parts().header} rows={parts().rows} />
      <Note>
        Not on this page yet: the applications installed and the ids that name this exact machine. They need a
        stronger capability than <C>System</C>.
      </Note>
      <H2>A report&apos;s header</H2>
      <P>
        From the spec: the line a bug report or a benchmark starts with. It prints this machine&apos;s details, so it
        is shown here, not run:
      </P>
      <CodeBlock code={specBlockWith(FILE, "14.25", "fn header()")} file="from spec 14.25" />
      <H2>Every function</H2>
      <ModuleTable file="25-system.md" number="14.25" first="Function" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Nothing here fails: what a system doesn&apos;t say comes back as <C>&quot;&quot;</C> or 0.</li>
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
        <SrcRef file="docs/decisions/0112-every-detail-of-the-machine.md" label="decision 0112" />
      </div>
    </>
  ),
};
