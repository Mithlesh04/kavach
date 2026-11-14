import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Running Programs ----------
//
// The samples run git, which every machine that builds this site has,
// and print only what is the same on all of them.

const FILE = "14-standard-library/23-process.md";
const TESTS = "tests/conformance/14-standard-library";

export const processDoc: LessonDoc = {
  lead: "std.process runs other programs: a compiler, a simulator, git, any tool with no library. It needs the capability Process, never goes through a shell, and ends everything a program started when it stops.",
  simple: <>Sometimes the tool you need is another program. <C>std.process</C> starts it for you, hands it its words exactly as you wrote them, and reads what it prints, like asking a colleague to run a job and report back.</>,
  minutes: 5,
  before: ["agents/capabilities", "errors/catch"],
  body: (
    <>
      <Ex name="process/run" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>process.run</C> runs a program to its end and gives a <C>Finished</C>: its exit code, what it printed (<C>out</C>, <C>err</C>) and what it cost (<C>cpu_ms</C>, <C>peak_bytes</C>).</li>
        <li><C>args</C> is a list, passed as it is, never through a shell. <C>&quot;--version; echo injected&quot;</C> is one odd argument, so git refuses it: nothing else runs.</li>
        <li>A program is a name looked up on the system&apos;s <C>PATH</C>, or a path such as <C>./tools/gen</C>.</li>
        <li><C>folder:</C> sets the folder it runs in, and <C>env:</C> adds or changes its environment variables. It reads nothing on standard input.</li>
      </ul>
      <H2>It needs Process</H2>
      <P>Running a program reaches outside your own, so the function says so in its <C>uses</C>, or it doesn&apos;t compile:</P>
      <Ex name="process/needs.fails" />
      <H2>Which programs</H2>
      <P>
        <C>Process</C> can be granted for one program, as <C>Fs</C> is for one folder. Inside a function with{" "}
        <C>uses Process(&quot;git&quot;)</C>, every program it runs, and everything it calls runs, must be git. Any other
        fails with <C>Denied</C>; a program that isn&apos;t there fails with <C>NotFound</C>:
      </P>
      <Ex name="process/names" />
      <ModuleTable file="23-process.md" number="14.23" first="Written" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The name is checked when the program runs, since it is a value.</li>
        <li>A name without a <C>/</C> matches only that name on <C>PATH</C>; a name with one matches only that path.</li>
        <li>Names nest, as folders do: inside two, a program must match both. So a function that runs two tools has <C>uses Process</C>, or calls one function per tool.</li>
        <li>A task keeps the names of the function that spawned it.</li>
      </ul>
      <H2>Output as it comes</H2>
      <P>
        <C>process.start</C> gives the program back running, as a <C>Child</C>. Its lines come one by one, each an{" "}
        <C>Output.Out(line)</C> or an <C>Output.Err(line)</C>, without its line end. <C>each</C> calls a function with
        each one:
      </P>
      <Ex name="process/lines" />
      <P><C>next()</C> reads one line, and <C>wait()</C> waits for the end, skipping what is still printed:</P>
      <Ex name="process/next" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An exit code of 0 most often means it went well. Git gives 129 for an option it doesn&apos;t know.</li>
        <li>On Linux and macOS, a program ended by a signal gives 128 plus the signal&apos;s number, as shells say it.</li>
        <li>Waiting for a line or for the end lets other tasks take their turns, so a program can run several tools at once, each in its own task.</li>
      </ul>
      <P>A longer one, from the spec: a simulator whose errors are picked out of its output.</P>
      <CodeBlock code={specBlockWith(FILE, "14.23", "process.start(\"vsim\"")} file="from spec 14.23" />
      <H2>A program is a tree</H2>
      <P>
        What a program starts is part of it. <C>kill()</C>, a <C>Child</C> nothing holds any more, a task cancelled
        while it waits, and a time limit (<C>.timeout</C>) all end the whole tree: through a job object on Windows,
        and a process group on Linux and macOS. So a timeout never leaves a simulator running behind it.
      </P>
      <H2>What it cost</H2>
      <P>
        <C>cpu_ms</C> and <C>peak_bytes</C> say how long a program used the processor and the most memory it held, so a
        tool that runs too long or grows too big can be seen. <C>process.peak_bytes()</C> says the same of your own
        program, and needs no capability:
      </P>
      <Ex name="process/peak" />
      <H2>Every function, method and type</H2>
      <ModuleTable file="23-process.md" number="14.23" first="Function or method" />
      <ModuleTable file="23-process.md" number="14.23" first="Type" />
      <Note>
        Not yet: text sent to a program&apos;s standard input, output read as <C>Bytes</C>, and a fake <C>Process</C>{" "}
        for <C>kavach test</C>. Until there is one, generated tests skip functions that use <C>Process</C>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.23 · std.process" />
        <SrcRef file={`${TESTS}/process-runs-a-program-with-no-shell.kav`} label="no shell, tested" />
        <SrcRef file={`${TESTS}/process-output-comes-line-by-line.kav`} label="line by line, tested" />
        <SrcRef file={`${TESTS}/process-names-narrow-the-programs.kav`} label="names, tested" />
        <SrcRef file={`${TESTS}/process-needs-process.kav`} label="Process, tested" />
        <SrcRef file="stdlib/process/process.kav" label="process.kav" />
        <SrcRef file="docs/decisions/0105-std-process.md" label="decision 0105" />
      </div>
    </>
  ),
};
